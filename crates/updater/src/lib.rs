// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use base64::Engine;
use http::HeaderName;
use minisign_verify::{PublicKey, Signature};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
    StatusCode,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    time::Duration,
};
use time::OffsetDateTime;
use url::Url;

use crate::current_exe::current_exe;

mod current_exe;
mod custom_serialization;
mod error;

pub use crate::error::*;
pub use http;
pub use reqwest;
pub use semver;
pub use url;

/// Install modes for the Windows update.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
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

impl WindowsUpdateInstallMode {
    /// Returns the associated `msiexec.exe` arguments.
    pub fn msiexec_args(&self) -> &'static [&'static str] {
        match self {
            Self::BasicUi => &["/qb+"],
            Self::Quiet => &["/quiet"],
            Self::Passive => &["/passive"],
        }
    }

    /// Returns the associated nsis arguments.
    pub fn nsis_args(&self) -> &'static [&'static str] {
        match self {
            Self::Passive => &["/P", "/R"],
            Self::Quiet => &["/S", "/R"],
            _ => &[],
        }
    }
}

/// The updater configuration for Windows.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct UpdaterWindowsConfig {
    /// Additional arguments given to the NSIS or WiX installer.
    pub installer_args: Vec<String>,
    /// The installation mode for the update on Windows. Defaults to `passive`.
    pub install_mode: WindowsUpdateInstallMode,
}

/// Updater configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub endpoints: Vec<Url>,
    /// Signature public key.
    pub pubkey: String,
    /// The Windows configuration for the updater.
    pub windows: UpdaterWindowsConfig,
}

/// Supported update format
#[derive(Debug, Serialize, Copy, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReleaseManifestPlatform {
    /// Download URL for the platform
    pub url: Url,
    /// Signature for the platform
    pub signature: String,
    /// Update format
    pub format: UpdateFormat,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RemoteReleaseInner {
    Dynamic(ReleaseManifestPlatform),
    Static {
        platforms: HashMap<String, ReleaseManifestPlatform>,
    },
}

/// Information about a release returned by the remote update server.
///
/// This type can have one of two shapes: Server Format (Dynamic Format) and Static Format.
#[derive(Debug, Clone)]
pub struct RemoteRelease {
    /// Version to install.
    pub version: Version,
    /// Release notes.
    pub notes: Option<String>,
    /// Release date.
    pub pub_date: Option<OffsetDateTime>,
    /// Release data.
    pub data: RemoteReleaseInner,
}

impl RemoteRelease {
    /// The release's download URL for the given target.
    pub fn download_url(&self, target: &str) -> Result<&Url> {
        match self.data {
            RemoteReleaseInner::Dynamic(ref platform) => Ok(&platform.url),
            RemoteReleaseInner::Static { ref platforms } => platforms
                .get(target)
                .map_or(Err(Error::TargetNotFound(target.to_string())), |p| {
                    Ok(&p.url)
                }),
        }
    }

    /// The release's signature for the given target.
    pub fn signature(&self, target: &str) -> Result<&String> {
        match self.data {
            RemoteReleaseInner::Dynamic(ref platform) => Ok(&platform.signature),
            RemoteReleaseInner::Static { ref platforms } => platforms
                .get(target)
                .map_or(Err(Error::TargetNotFound(target.to_string())), |platform| {
                    Ok(&platform.signature)
                }),
        }
    }

    /// The release's update format for the given target.
    pub fn format(&self, target: &str) -> Result<UpdateFormat> {
        match self.data {
            RemoteReleaseInner::Dynamic(ref platform) => Ok(platform.format),
            RemoteReleaseInner::Static { ref platforms } => platforms
                .get(target)
                .map_or(Err(Error::TargetNotFound(target.to_string())), |platform| {
                    Ok(platform.format)
                }),
        }
    }
}

pub struct UpdaterBuilder {
    current_version: Version,
    config: Config,
    version_comparator: Option<Box<dyn Fn(Version, RemoteRelease) -> bool + Send + Sync>>,
    executable_path: Option<PathBuf>,
    target: Option<String>,
    headers: HeaderMap,
    timeout: Option<Duration>,
}

impl UpdaterBuilder {
    pub fn new(current_version: Version, config: crate::Config) -> Self {
        Self {
            current_version,
            config,
            version_comparator: None,
            executable_path: None,
            target: None,
            headers: Default::default(),
            timeout: None,
        }
    }

    pub fn version_comparator<F: Fn(Version, RemoteRelease) -> bool + Send + Sync + 'static>(
        mut self,
        f: F,
    ) -> Self {
        self.version_comparator = Some(Box::new(f));
        self
    }

    pub fn pub_key(mut self, pub_key: impl Into<String>) -> Self {
        self.config.pubkey = pub_key.into();
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target.replace(target.into());
        self
    }

    pub fn endpoints(mut self, endpoints: Vec<Url>) -> Self {
        self.config.endpoints = endpoints;
        self
    }

    pub fn executable_path<P: AsRef<Path>>(mut self, p: P) -> Self {
        self.executable_path.replace(p.as_ref().into());
        self
    }

    pub fn header<K, V>(mut self, key: K, value: V) -> Result<Self>
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        let key: std::result::Result<HeaderName, http::Error> = key.try_into().map_err(Into::into);
        let value: std::result::Result<HeaderValue, http::Error> =
            value.try_into().map_err(Into::into);
        self.headers.insert(key?, value?);

        Ok(self)
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn installer_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.windows.installer_args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn build(self) -> Result<Updater> {
        if self.config.endpoints.is_empty() {
            return Err(Error::EmptyEndpoints);
        };

        let arch = get_updater_arch().ok_or(Error::UnsupportedArch)?;
        let (target, json_target) = if let Some(target) = self.target {
            (target.clone(), target)
        } else {
            let target = get_updater_target().ok_or(Error::UnsupportedOs)?;
            (target.to_string(), format!("{target}-{arch}"))
        };

        let executable_path = self.executable_path.clone().unwrap_or(current_exe()?);

        // Get the extract_path from the provided executable_path
        let extract_path = if cfg!(target_os = "linux") {
            executable_path
        } else {
            extract_path_from_executable(&executable_path)?
        };

        Ok(Updater {
            config: self.config,
            current_version: self.current_version,
            version_comparator: self.version_comparator,
            timeout: self.timeout,
            arch,
            target,
            json_target,
            headers: self.headers,
            extract_path,
        })
    }
}

pub struct Updater {
    config: Config,
    current_version: Version,
    version_comparator: Option<Box<dyn Fn(Version, RemoteRelease) -> bool + Send + Sync>>,
    timeout: Option<Duration>,
    arch: &'static str,
    // The `{{target}}` variable we replace in the endpoint
    target: String,
    // The value we search if the updater server returns a JSON with the `platforms` object
    json_target: String,
    headers: HeaderMap,
    extract_path: PathBuf,
}

impl Updater {
    pub fn check(&self) -> Result<Option<Update>> {
        // we want JSON only
        let mut headers = self.headers.clone();
        headers.insert("Accept", HeaderValue::from_str("application/json").unwrap());

        // Set SSL certs for linux if they aren't available.
        #[cfg(target_os = "linux")]
        {
            if std::env::var_os("SSL_CERT_FILE").is_none() {
                std::env::set_var("SSL_CERT_FILE", "/etc/ssl/certs/ca-certificates.crt");
            }
            if std::env::var_os("SSL_CERT_DIR").is_none() {
                std::env::set_var("SSL_CERT_DIR", "/etc/ssl/certs");
            }
        }

        let mut remote_release: Option<RemoteRelease> = None;
        let mut last_error: Option<Error> = None;
        for url in &self.config.endpoints {
            // replace {{current_version}}, {{target}} and {{arch}} in the provided URL
            // this is useful if we need to query example
            // https://releases.myapp.com/update/{{target}}/{{arch}}/{{current_version}}
            // will be translated into ->
            // https://releases.myapp.com/update/macos/aarch64/1.0.0
            // The main objective is if the update URL is defined via the Cargo.toml
            // the URL will be generated dynamically
            let url: Url = url
                .to_string()
                // url::Url automatically url-encodes the string
                .replace(
                    "%7B%7Bcurrent_version%7D%7D",
                    &self.current_version.to_string(),
                )
                .replace("%7B%7Btarget%7D%7D", &self.target)
                .replace("%7B%7Barch%7D%7D", self.arch)
                .parse()?;

            let mut request = Client::new().get(url).headers(headers.clone());
            if let Some(timeout) = self.timeout {
                request = request.timeout(timeout);
            }
            let response = request.send();

            if let Ok(res) = response {
                if res.status().is_success() {
                    // no updates found!
                    if StatusCode::NO_CONTENT == res.status() {
                        return Ok(None);
                    };

                    match serde_json::from_value::<RemoteRelease>(res.json()?).map_err(Into::into) {
                        Ok(release) => {
                            last_error = None;
                            remote_release = Some(release);
                            // we found a relase, break the loop
                            break;
                        }
                        Err(err) => last_error = Some(err),
                    }
                }
            }
        }

        // Last error is cleaned on success.
        // Shouldn't be triggered if we had a successfull call
        if let Some(error) = last_error {
            return Err(error);
        }

        // Extracted remote metadata
        let release = remote_release.ok_or(Error::ReleaseNotFound)?;

        let should_update = match self.version_comparator.as_ref() {
            Some(comparator) => comparator(self.current_version.clone(), release.clone()),
            None => release.version > self.current_version,
        };

        let update = if should_update {
            Some(Update {
                current_version: self.current_version.to_string(),
                config: self.config.clone(),
                target: self.target.clone(),
                extract_path: self.extract_path.clone(),
                version: release.version.to_string(),
                date: release.pub_date,
                download_url: release.download_url(&self.json_target)?.to_owned(),
                body: release.notes.clone(),
                signature: release.signature(&self.json_target)?.to_owned(),
                timeout: self.timeout,
                headers: self.headers.clone(),
                format: release.format(&self.json_target)?,
            })
        } else {
            None
        };

        Ok(update)
    }
}

#[derive(Debug, Clone)]
pub struct Update {
    config: Config,
    /// Update description
    pub body: Option<String>,
    /// Version used to check for update
    pub current_version: String,
    /// Version announced
    pub version: String,
    /// Update publish date
    pub date: Option<OffsetDateTime>,
    /// Target
    pub target: String,
    /// Extract path
    #[allow(unused)]
    extract_path: PathBuf,
    /// Download URL announced
    pub download_url: Url,
    /// Signature announced
    pub signature: String,
    /// Request timeout
    pub timeout: Option<Duration>,
    /// Request headers
    pub headers: HeaderMap,
    /// Update format
    pub format: UpdateFormat,
}

impl Update {
    /// Downloads the updater package, verifies it then return it as bytes.
    ///
    /// Use [`Update::install`] to install it
    pub fn download<C: Fn(usize, Option<u64>), D: FnOnce()>(
        &self,
        on_chunk: C,
        on_download_finish: D,
    ) -> Result<Vec<u8>> {
        // set our headers
        let mut headers = self.headers.clone();
        headers.insert(
            "Accept",
            HeaderValue::from_str("application/octet-stream").unwrap(),
        );
        headers.insert(
            "User-Agent",
            HeaderValue::from_str("cargo-packager-updater").unwrap(),
        );

        let mut request = Client::new()
            .get(self.download_url.clone())
            .headers(headers);
        if let Some(timeout) = self.timeout {
            request = request.timeout(timeout);
        }

        struct DownloadProgress<R, C: Fn(usize, Option<u64>)> {
            content_length: Option<u64>,
            inner: R,
            on_chunk: C,
        }

        impl<R: Read, C: Fn(usize, Option<u64>)> Read for DownloadProgress<R, C> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.inner.read(buf).map(|n| {
                    (self.on_chunk)(n, self.content_length);
                    n
                })
            }
        }

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(Error::Network(format!(
                "Download request failed with status: {}",
                response.status()
            )));
        }

        let mut source = DownloadProgress {
            content_length: response
                .headers()
                .get("Content-Length")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse().ok()),
            inner: response,
            on_chunk,
        };

        let mut buffer = Vec::new();

        let _ = std::io::copy(&mut source, &mut buffer)?;
        on_download_finish();

        let mut update_buffer = Cursor::new(&buffer);

        verify_signature(&mut update_buffer, &self.signature, &self.config.pubkey)?;

        Ok(buffer)
    }

    /// Installs the updater package downloaded by [`Update::download`]
    pub fn install(&self, bytes: Vec<u8>) -> Result<()> {
        self.install_inner(bytes)
    }

    /// Downloads and installs the updater package
    pub fn download_and_install<C: Fn(usize, Option<u64>), D: FnOnce()>(
        &self,
        on_chunk: C,
        on_download_finish: D,
    ) -> Result<()> {
        let bytes = self.download(on_chunk, on_download_finish)?;
        self.install(bytes)
    }

    // Windows
    //
    // ### Expected installers:
    // │── [AppName]_[version]_x64.msi           # Application MSI
    // │── [AppName]_[version]_x64-setup.exe           # NSIS installer
    // └── ...
    #[cfg(windows)]
    fn install_inner(&self, bytes: Vec<u8>) -> Result<()> {
        use std::{io::Write, process::Command};

        let extension = match self.format {
            UpdateFormat::Nsis => ".exe",
            UpdateFormat::Wix => ".msi",
            _ => return Err(crate::Error::UnsupportedUpdateFormat),
        };

        let mut temp_file = tempfile::Builder::new().suffix(extension).tempfile()?;
        temp_file.write_all(&bytes)?;
        let (f, path) = temp_file.keep()?;
        drop(f);

        let system_root = std::env::var("SYSTEMROOT");
        let powershell_path = system_root.as_ref().map_or_else(
            |_| "powershell.exe".to_string(),
            |p| format!("{p}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"),
        );

        // we support 2 type of files exe & msi for now
        // If it's an `exe` we expect an installer not a runtime.
        match self.format {
            UpdateFormat::Nsis => {
                // we need to wrap the installer path in quotes for Start-Process
                let mut installer_arg = std::ffi::OsString::new();
                installer_arg.push("\"");
                installer_arg.push(&path);
                installer_arg.push("\"");

                // Run the installer
                Command::new(powershell_path)
                    .args(["-NoProfile", "-WindowStyle", "Hidden"])
                    .args(["Start-Process"])
                    .arg(installer_arg)
                    .arg("-ArgumentList")
                    .arg(
                        [
                            self.config.windows.install_mode.nsis_args(),
                            self.config
                                .windows
                                .installer_args
                                .iter()
                                .map(AsRef::as_ref)
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ]
                        .concat()
                        .join(", "),
                    )
                    .spawn()
                    .expect("installer failed to start");

                std::process::exit(0);
            }
            UpdateFormat::Wix => {
                {
                    // we need to wrap the current exe path in quotes for Start-Process
                    let mut current_exe_arg = std::ffi::OsString::new();
                    current_exe_arg.push("\"");
                    current_exe_arg.push(current_exe()?);
                    current_exe_arg.push("\"");

                    let mut msi_path_arg = std::ffi::OsString::new();
                    msi_path_arg.push("\"\"\"");
                    msi_path_arg.push(&path);
                    msi_path_arg.push("\"\"\"");

                    let msiexec_args = self
                        .config
                        .windows
                        .install_mode
                        .msiexec_args()
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<String>>();

                    // run the installer and relaunch the application
                    let powershell_install_res = Command::new(powershell_path)
                        .args(["-NoProfile", "-WindowStyle", "Hidden"])
                        .args([
                            "Start-Process",
                            "-Wait",
                            "-FilePath",
                            "$env:SYSTEMROOT\\System32\\msiexec.exe",
                            "-ArgumentList",
                        ])
                        .arg("/i,")
                        .arg(&msi_path_arg)
                        .arg(format!(", {}, /promptrestart;", msiexec_args.join(", ")))
                        .arg("Start-Process")
                        .arg(current_exe_arg)
                        .spawn();
                    if powershell_install_res.is_err() {
                        // fallback to running msiexec directly - relaunch won't be available
                        // we use this here in case powershell fails in an older machine somehow
                        let msiexec_path = system_root.as_ref().map_or_else(
                            |_| "msiexec.exe".to_string(),
                            |p| format!("{p}\\System32\\msiexec.exe"),
                        );
                        let _ = Command::new(msiexec_path)
                            .arg("/i")
                            .arg(msi_path_arg)
                            .args(msiexec_args)
                            .arg("/promptrestart")
                            .spawn();
                    }

                    std::process::exit(0);
                }
            }
            _ => unreachable!(),
        }
    }

    // Linux (AppImage)
    //
    // ### Expected structure:
    // ├── [AppName]_[version]_amd64.AppImage.tar.gz    # GZ generated by cargo-packager
    // │   └──[AppName]_[version]_amd64.AppImage        # Application AppImage
    // └── ...
    //
    // We should have an AppImage already installed to be able to copy and install
    // the extract_path is the current AppImage path
    // tmp_dir is where our new AppImage is found
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    fn install_inner(&self, bytes: Vec<u8>) -> Result<()> {
        match self.format {
            UpdateFormat::AppImage => {}
            _ => return Err(crate::Error::UnsupportedUpdateFormat),
        };

        let extract_path_metadata = self.extract_path.metadata()?;
        let tmp_dir_locations = vec![
            Box::new(|| Some(std::env::temp_dir())) as Box<dyn FnOnce() -> Option<PathBuf>>,
            Box::new(dirs::cache_dir),
            Box::new(|| Some(self.extract_path.parent().unwrap().to_path_buf())),
        ];

        for tmp_dir_location in tmp_dir_locations {
            if let Some(tmp_dir) = tmp_dir_location() {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};

                let tmp_dir_metadata = tmp_dir.metadata()?;
                if extract_path_metadata.dev() == tmp_dir_metadata.dev() {
                    let mut perms = tmp_dir_metadata.permissions();
                    perms.set_mode(0o700);
                    std::fs::set_permissions(&tmp_dir, perms)?;

                    let (_, tmp_app_image) = tempfile::Builder::new()
                        .prefix("current_app")
                        .suffix(".AppImage")
                        .tempfile_in(tmp_dir)?
                        .keep()?;

                    // get metadata to restore later
                    let metadata = self.extract_path.metadata()?;

                    // create a backup of our current app image
                    std::fs::rename(&self.extract_path, &tmp_app_image)?;

                    // if something went wrong during the extraction, we should restore previous app
                    if let Err(err) = std::fs::write(&self.extract_path, bytes).and_then(|_| {
                        std::fs::set_permissions(&self.extract_path, metadata.permissions())
                    }) {
                        std::fs::rename(tmp_app_image, &self.extract_path)?;
                        return Err(err.into());
                    }

                    // early finish we have everything we need here
                    return Ok(());
                }
            }
        }

        Err(Error::TempDirNotOnSameMountPoint)
    }

    // MacOS
    //
    // ### Expected structure:
    // ├── [AppName]_[version]_x64.app.tar.gz       # GZ generated by cargo-packager
    // │   └──[AppName].app                         # Main application
    // │      └── Contents                          # Application contents...
    // │          └── ...
    // └── ...
    #[cfg(target_os = "macos")]
    fn install_inner(&self, bytes: Vec<u8>) -> Result<()> {
        use flate2::read::GzDecoder;

        let cursor = Cursor::new(bytes);
        let mut extracted_files: Vec<PathBuf> = Vec::new();

        // the first file in the tar.gz will always be
        // <app_name>/Contents
        let tmp_dir = tempfile::Builder::new().prefix("current_app").tempdir()?;

        // create backup of our current app
        std::fs::rename(&self.extract_path, tmp_dir.path())?;

        let decoder = GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);

        std::fs::create_dir(&self.extract_path)?;

        for entry in archive.entries()? {
            let mut entry = entry?;

            let extraction_path = &self.extract_path.join(entry.path()?);

            // if something went wrong during the extraction, we should restore previous app
            if let Err(err) = entry.unpack(extraction_path) {
                for file in extracted_files.iter().rev() {
                    // delete all the files we extracted
                    if file.is_dir() {
                        std::fs::remove_dir(file)?;
                    } else {
                        std::fs::remove_file(file)?;
                    }
                }
                std::fs::rename(tmp_dir.path(), &self.extract_path)?;
                return Err(err.into());
            }

            extracted_files.push(extraction_path.to_path_buf());
        }

        let _ = std::process::Command::new("touch")
            .arg(&self.extract_path)
            .status();

        Ok(())
    }
}

/// Gets the target string used on the updater.
pub fn target() -> Option<String> {
    if let (Some(target), Some(arch)) = (get_updater_target(), get_updater_arch()) {
        Some(format!("{target}-{arch}"))
    } else {
        None
    }
}

pub(crate) fn get_updater_target() -> Option<&'static str> {
    if cfg!(target_os = "linux") {
        Some("linux")
    } else if cfg!(target_os = "macos") {
        Some("macos")
    } else if cfg!(target_os = "windows") {
        Some("windows")
    } else {
        None
    }
}

pub(crate) fn get_updater_arch() -> Option<&'static str> {
    if cfg!(target_arch = "x86") {
        Some("i686")
    } else if cfg!(target_arch = "x86_64") {
        Some("x86_64")
    } else if cfg!(target_arch = "arm") {
        Some("armv7")
    } else if cfg!(target_arch = "aarch64") {
        Some("aarch64")
    } else {
        None
    }
}

pub fn extract_path_from_executable(executable_path: &Path) -> Result<PathBuf> {
    // Return the path of the current executable by default
    // Example C:\Program Files\My App\
    let extract_path = executable_path
        .parent()
        .map(PathBuf::from)
        .ok_or(Error::FailedToDetermineExtractPath)?;

    // MacOS example binary is in /Applications/TestApp.app/Contents/MacOS/myApp
    // We need to get /Applications/<app>.app
    // TODO(lemarier): Need a better way here
    // Maybe we could search for <*.app> to get the right path
    #[cfg(target_os = "macos")]
    if extract_path
        .display()
        .to_string()
        .contains("Contents/MacOS")
    {
        return extract_path
            .parent()
            .map(PathBuf::from)
            .ok_or(Error::FailedToDetermineExtractPath)?
            .parent()
            .map(PathBuf::from)
            .ok_or(Error::FailedToDetermineExtractPath);
    }

    Ok(extract_path)
}

// Validate signature
// need to be public because its been used
// by our tests in the bundler
//
// NOTE: The buffer position is not reset.
pub fn verify_signature<R>(
    archive_reader: &mut R,
    release_signature: &str,
    pub_key: &str,
) -> Result<bool>
where
    R: Read,
{
    // we need to convert the pub key
    let pub_key_decoded = base64_to_string(pub_key)?;
    let public_key = PublicKey::decode(&pub_key_decoded)?;
    let signature_base64_decoded = base64_to_string(release_signature)?;
    let signature = Signature::decode(&signature_base64_decoded)?;

    // read all bytes until EOF in the buffer
    let mut data = Vec::new();
    archive_reader.read_to_end(&mut data)?;

    // Validate signature or bail out
    public_key.verify(&data, &signature, true)?;
    Ok(true)
}

fn base64_to_string(base64_string: &str) -> Result<String> {
    let decoded_string = &base64::engine::general_purpose::STANDARD.decode(base64_string)?;
    let result = std::str::from_utf8(decoded_string)
        .map_err(|_| Error::SignatureUtf8(base64_string.into()))?
        .to_string();
    Ok(result)
}
