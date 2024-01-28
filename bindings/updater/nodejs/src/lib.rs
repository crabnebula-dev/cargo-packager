use std::{collections::HashMap, str::FromStr, time::Duration};

use cargo_packager_updater::{
    http::{HeaderMap, HeaderName, HeaderValue},
    semver::Version,
    Updater, UpdaterBuilder,
};
use napi::{
    bindgen_prelude::AsyncTask,
    threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
    Env, Error, JsArrayBuffer, Result, Status, Task,
};

mod from_impls;

#[napi_derive::napi(string_enum)]
#[derive(Default)]
pub enum WindowsUpdateInstallMode {
    /// Specifies there's a basic UI during the installation process, including a final dialog box at the end.
    BasicUi,
    /// The quiet mode means there's no user interaction required.
    /// Requires admin privileges if the installer does.
    Quiet,
    /// Specifies unattended mode, which means the installation only shows a progress bar.
    #[default]
    Passive,
}

#[derive(Clone)]
#[napi_derive::napi(object)]
pub struct UpdaterWindowsOptions {
    /// Additional arguments given to the NSIS or WiX installer.
    pub installer_args: Option<Vec<String>>,
    /// The installation mode for the update on Windows. Defaults to `passive`.
    pub install_mode: Option<WindowsUpdateInstallMode>,
}

#[napi_derive::napi(object)]
pub struct Options {
    /// The updater endpoints.
    pub endpoints: Vec<String>,
    /// Signature public key.
    pub pubkey: String,
    /// The Windows options for the updater.
    pub windows: Option<UpdaterWindowsOptions>,
    /// The target of the executable.
    pub target: Option<String>,
    /// Path to the executable file.
    pub executable_path: Option<String>,
    /// Headers to use when checking and when downloading the update.
    pub headers: Option<HashMap<String, String>>,
    /// Request timeout in milliseconds.
    pub timeout: Option<u32>,
}

impl Options {
    fn into_updater(mut self, current_version: Version) -> Result<Updater> {
        let target = self.target.take();
        let executable_path = self.executable_path.take();
        let headers = self.headers.take();
        let timeout = self.timeout.take();
        let config: cargo_packager_updater::Config = self.into();

        let mut builder = UpdaterBuilder::new(current_version, config);
        if let Some(target) = target {
            builder = builder.target(target);
        }
        if let Some(executable_path) = executable_path {
            builder = builder.executable_path(executable_path);
        }
        if let Some(timeout) = timeout {
            builder = builder.timeout(Duration::from_millis(timeout as u64));
        }
        if let Some(headers) = headers {
            for (key, value) in headers {
                builder = builder.header(key, value).map_err(|e| {
                    Error::new(
                        Status::InvalidArg,
                        format!("Failed to set header, probably invalid header values, {e}"),
                    )
                })?;
            }
        }

        builder.build().map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to construct updater, {e}"),
            )
        })
    }
}

/// Supported update format
#[napi_derive::napi]
pub enum UpdateFormat {
    /// The NSIS installer (.exe).
    Nsis,
    /// The Microsoft Software Installer (.msi) through WiX Toolset.
    Wix,
    /// The Linux AppImage package (.AppImage).
    AppImage,
    /// The macOS application bundle (.app).
    App,
}

#[napi_derive::napi]
pub struct Update {
    /// Signing public key
    pub pubkey: String,
    /// Version used to check for update
    pub current_version: String,
    /// Version announced
    pub version: String,
    /// Target
    pub target: String,
    /// Extract path
    pub extract_path: String,
    /// Download URL announced
    pub download_url: String,
    /// Signature announced
    pub signature: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Update format
    pub format: UpdateFormat,
    /// The Windows options for the updater.
    pub windows: Option<UpdaterWindowsOptions>,
    /// Update description
    pub body: Option<String>,
    /// Update publish date
    pub date: Option<String>,
    /// Request timeout
    pub timeout: Option<u32>,
}

impl Update {
    fn create_update(&self) -> Result<cargo_packager_updater::Update> {
        Ok(cargo_packager_updater::Update {
            config: cargo_packager_updater::Config {
                pubkey: self.pubkey.clone(),
                windows: self.windows.clone().map(Into::into),
                ..Default::default()
            },
            body: self.body.clone(),
            current_version: self.current_version.clone(),
            version: self.version.clone(),
            date: None,
            target: self.target.clone(),
            extract_path: self.extract_path.clone().into(),
            download_url: self.download_url.parse().map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Internal error, couldn't convert string to Url struct, {e}"),
                )
            })?,
            signature: self.signature.clone(),
            timeout: self.timeout.map(|t| Duration::from_millis(t as u64)),
            headers: {
                let mut map = HeaderMap::new();
                for (key, value) in &self.headers {
                    map.insert(HeaderName::from_str(key).map_err(|e| {
                        Error::new(
                            Status::GenericFailure,
                            format!("Internal error, couldn't construct header name from str , {e}"),
                        )
                    })?, HeaderValue::from_str(value).map_err(|e| {
                        Error::new(
                            Status::GenericFailure,
                            format!("Internal error, couldn't construct header value from str , {e}"),
                        )
                    })?);
                }

                map
            },
            format: self.format.into(),
        })
    }
}

type TaskCallbackFunction<T> = Option<ThreadsafeFunction<T, ErrorStrategy::Fatal>>;

pub struct DownloadTask {
    update: cargo_packager_updater::Update,
    on_chunk: TaskCallbackFunction<(u32, Option<u32>)>,
    on_download_finished: TaskCallbackFunction<()>,
}

impl DownloadTask {
    pub fn create(
        update: &Update,
        on_chunk: TaskCallbackFunction<(u32, Option<u32>)>,
        on_download_finished: TaskCallbackFunction<()>,
    ) -> Result<Self> {
        Ok(Self {
            update: update.create_update()?,
            on_chunk,
            on_download_finished,
        })
    }
}

impl Task for DownloadTask {
    type Output = Vec<u8>;
    type JsValue = JsArrayBuffer;

    fn compute(&mut self) -> Result<Self::Output> {
        let on_chunk = |chunk_len: usize, content_len: Option<u64>| {
            if let Some(on_chunk) = &self.on_chunk {
                on_chunk.call(
                    (chunk_len as _, content_len.map(|v| v as _)),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
        };

        let on_finish = || {
            if let Some(on_download_finished) = &self.on_download_finished {
                on_download_finished.call((), ThreadsafeFunctionCallMode::NonBlocking);
            }
        };

        self.update
            .download_extended(on_chunk, on_finish)
            .map_err(|e| Error::new(Status::GenericFailure, e))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        let mut buffer = env.create_arraybuffer(output.len())?;
        unsafe { std::ptr::copy(output.as_ptr(), buffer.as_mut_ptr(), output.len()) };

        Ok(buffer.into_raw())
    }
}

pub struct InstallTask {
    update: cargo_packager_updater::Update,
    bytes: Option<Vec<u8>>,
}

impl InstallTask {
    pub fn create(update: &Update, bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            update: update.create_update()?,
            bytes: Some(bytes),
        })
    }
}

impl Task for InstallTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> Result<Self::Output> {
        self.update
            .install(self.bytes.take().unwrap())
            .map_err(|e| Error::new(Status::GenericFailure, e))
    }

    fn resolve(&mut self, _env: Env, _output: Self::Output) -> Result<Self::JsValue> {
        Ok(())
    }
}

pub struct DownloadAndInstallTask {
    download_task: DownloadTask,
}

impl DownloadAndInstallTask {
    pub fn new(download_task: DownloadTask) -> Self {
        Self { download_task }
    }
}

impl Task for DownloadAndInstallTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> Result<Self::Output> {
        let bytes = self.download_task.compute()?;
        self.download_task
            .update
            .install(bytes)
            .map_err(|e| Error::new(Status::GenericFailure, e))
    }

    fn resolve(&mut self, _env: Env, _output: Self::Output) -> Result<Self::JsValue> {
        Ok(())
    }
}

pub struct CheckUpdateTask {
    updater: Updater,
}

impl CheckUpdateTask {
    pub fn create(current_version: String, options: Options) -> Result<Self> {
        let current_version = current_version.parse().map_err(|e| {
            Error::new(
                Status::InvalidArg,
                format!("Failed to parse string as a valid semver, {e}"),
            )
        })?;

        let updater = options.into_updater(current_version)?;

        Ok(Self { updater })
    }
}

impl Task for CheckUpdateTask {
    type Output = Option<cargo_packager_updater::Update>;
    type JsValue = Option<Update>;

    fn compute(&mut self) -> Result<Self::Output> {
        self.updater.check().map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to check for update, {e}"),
            )
        })
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.map(Into::into))
    }
}

#[napi_derive::napi]
impl Update {
    #[napi(
        ts_args_type = "onChunk?: (chunkLength: number, contentLength: number | null) => void, onDownloadFinished?: () => void",
        ts_return_type = "Promise<ArrayBuffer>"
    )]
    pub fn download(
        &self,
        on_chunk: TaskCallbackFunction<(u32, Option<u32>)>,
        on_download_finish: TaskCallbackFunction<()>,
    ) -> Result<AsyncTask<DownloadTask>> {
        DownloadTask::create(self, on_chunk, on_download_finish).map(AsyncTask::new)
    }

    #[napi(ts_return_type = "Promise<void>", ts_args_type = "buffer: ArrayBuffer")]
    pub fn install(&self, bytes: JsArrayBuffer) -> Result<AsyncTask<InstallTask>> {
        let bytes = bytes.into_value()?;
        let bytes = bytes.as_ref().to_vec();
        InstallTask::create(self, bytes).map(AsyncTask::new)
    }

    #[napi(
        ts_args_type = "onChunk?: (chunkLength: number, contentLength?: number) => void, onDownloadFinished?: () => void",
        ts_return_type = "Promise<void>"
    )]
    pub fn download_and_install(
        &self,
        on_chunk: TaskCallbackFunction<(u32, Option<u32>)>,
        on_download_finish: TaskCallbackFunction<()>,
    ) -> Result<AsyncTask<DownloadAndInstallTask>> {
        let download_task = DownloadTask::create(self, on_chunk, on_download_finish)?;
        Ok(AsyncTask::new(DownloadAndInstallTask::new(download_task)))
    }
}

#[napi_derive::napi(ts_return_type = "Promise<Update | null>")]
pub fn check_update(
    current_version: String,
    options: Options,
) -> Result<AsyncTask<CheckUpdateTask>> {
    Ok(AsyncTask::new(CheckUpdateTask::create(
        current_version,
        options,
    )?))
}
