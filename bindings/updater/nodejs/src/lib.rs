use std::{collections::HashMap, str::FromStr, time::Duration};

use cargo_packager_updater::{
    http::{HeaderMap, HeaderName, HeaderValue},
    semver::Version,
    Updater, UpdaterBuilder,
};
use napi::{
    threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
    Error, Result, Status,
};

mod from_impls;

#[napi_derive::napi]
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
    pub installer_args: Vec<String>,
    /// The installation mode for the update on Windows. Defaults to `passive`.
    pub install_mode: WindowsUpdateInstallMode,
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
    pub windows: UpdaterWindowsOptions,
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
                windows: self.windows.clone().into(),
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

#[napi_derive::napi]
impl Update {
    #[napi(
        ts_args_type = "onChunk?: (chunkLength: number, contentLength: number | null) => void, onDownloadFinished?: () => void"
    )]
    pub async fn download(
        &self,
        on_chunk: Option<ThreadsafeFunction<(u32, Option<u32>), ErrorStrategy::CalleeHandled>>,
        on_download_finish: Option<ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>>,
    ) -> Result<Vec<u8>> {
        let update = self.create_update()?;

        update
            .download(
                |c, l| {
                    if let Some(on_chunk) = &on_chunk {
                        on_chunk.call(
                            Ok((c as u32, l.map(|l| l as u32))),
                            ThreadsafeFunctionCallMode::Blocking,
                        );
                    }
                },
                || {
                    if let Some(on_download_finish) = on_download_finish {
                        on_download_finish.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
                    }
                },
            )
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub async fn install(&self, bytes: Vec<u8>) -> Result<()> {
        let update = self.create_update()?;
        update
            .install(bytes)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi(
        ts_args_type = "onChunk?: (chunkLength: number, contentLength: number | null) => void, onDownloadFinished?: () => void"
    )]
    pub async fn download_and_install(
        &self,
        on_chunk: Option<ThreadsafeFunction<(u32, Option<u32>), ErrorStrategy::CalleeHandled>>,
        on_download_finish: Option<ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>>,
    ) -> Result<()> {
        let update = self.create_update()?;
        let bytes = update
            .download(
                |c, l| {
                    if let Some(on_chunk) = &on_chunk {
                        on_chunk.call(
                            Ok((c as u32, l.map(|l| l as u32))),
                            ThreadsafeFunctionCallMode::Blocking,
                        );
                    }
                },
                || {
                    if let Some(on_download_finish) = on_download_finish {
                        on_download_finish.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
                    }
                },
            )
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;
        update
            .install(bytes)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }
}

#[napi_derive::napi]
pub async fn check_update(current_version: String, options: Options) -> Result<Option<Update>> {
    let current_version = current_version.parse().map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to parse string as a valid semver, {e}"),
        )
    })?;

    let updater = options.into_updater(current_version)?;

    let update = updater.check().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to check for update, {e}"),
        )
    })?;

    Ok(update.map(Into::into))
}
