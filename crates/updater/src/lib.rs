// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! # cargo-packager-updater
//!
//! Updater for apps that was packaged by [`cargo-packager`](https://docs.rs/cargo-packager).
//!
//! ## Checking for an update
//!
//! you can check for an update using [`check_update`] function or construct a new [`Updater`]
//! using [`UpdaterBuilder`], both methods require the current version of the app and
//! a [`Config`] that specifies the endpoints to request updates from and the public key of the update signature.
//!
//! ```no_run
//! use cargo_packager_updater::{check_update, Config};
//!
//! let config = Config {
//!   endpoints: vec!["http://myserver.com/updates".parse().unwrap()],
//!   pubkey: "<pubkey here>".into(),
//!   ..Default::default()
//! };
//! if let Some(update) = check_update("0.1.0".parse().unwrap(), config).expect("failed while checking for update") {
//!     update.download_and_install().expect("failed to download and install update");
//! } else {
//!     // there is no updates
//! }
//!
//! ```
//!
//! ## Endpoints
//!
//! Each endpoint optionally could have `{{arch}}`, `{{target}}` or `{{current_version}}`
//! which will be detected and replaced with the appropriate value before making a request to the endpoint.
//!
//! - `{{current_version}}`: The version of the app that is requesting the update.
//! - `{{target}}`: The operating system name (one of `linux`, `windows` or `macos`).
//! - `{{arch}}`: The architecture of the machine (one of `x86_64`, `i686`, `aarch64` or `armv7`).
//!
//! for example:
//! ```text
//! "https://releases.myapp.com/{{target}}/{{arch}}/{{current_version}}"
//! ```
//! will turn into
//! ```text
//! "https://releases.myapp.com/windows/x86_64/0.1.0"
//! ```
//!
//! if you need more data, you can set additional request headers [`UpdaterBuilder::header`] to your liking.
//!
//! ## Endpoint Response
//!
//! The updater expects the endpoint to respond with 2 possible responses:
//!
//! 1. [`204 No Content`](https://datatracker.ietf.org/doc/html/rfc2616#section-10.2.5) in case there is no updates available.
//! 2. [`200 OK`](https://datatracker.ietf.org/doc/html/rfc2616#section-10.2.1) and a JSON response that could be either a JSON representing all available platform updates
//!    or if using endpoints variables (see above) or a header to attach the current updater target,
//!    then it can just return information for the requested target.
//!
//! The JSON response is expected to have these fields set:
//!
//! - `version`: must be a valid semver, with or without a leading `v``, meaning that both `1.0.0` and `v1.0.0` are valid.
//! - `url` or `platforms.[target].url`: must be a valid url to the update bundle
//! - `signature` or `platforms.[target].signature`: must be the content of the generated `.sig` file. The signature may change each time you run build your app so make sure to always update it.
//! - `format` or `platforms.[target].format`: must be one of `app`, `appimage`, `nsis` or `wix`.
//!
//! <div style="border-left: 2px solid rgba(47,129,247);padding-left:0.75em;">
//!   <p style="display:flex;align-items:center;gap:3px;color:rgb(47,129,247)">
//!     <svg viewBox="0 0 16 16" version="1.1" width="16" height="16" aria-hidden="true"><path fill="rgb(47,129,247)" d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"></path></svg>
//!     Note
//!   </p>
//!   if using <code>platforms</code> object, each key is in the <code>OS-ARCH</code> format, where <code>OS</code> is one of <code>linux</code>, <code>macos</code> or <code>windows</code>, and <code>ARCH</code> is one of <code>x86_64</code>, <code>aarch64</code>, <code>i686</code> or <code>armv7</code>, see the example below.
//! </div>
//! <br>
//!
//! It can also contain these optional fields:
//! - `notes`: Here you can add notes about the update, like release notes.
//! - `pub_date`: must be formatted according to [RFC 3339](https://datatracker.ietf.org/doc/html/rfc3339#section-5.8) if present.
//!
//! Here is an example of the two expected JSON formats:
//!
//!  - **JSON for all platforms**
//!
//!    ```json
//!    {
//!      "version": "v1.0.0",
//!      "notes": "Test version",
//!      "pub_date": "2020-06-22T19:25:57Z",
//!      "platforms": {
//!        "macos-x86_64": {
//!          "signature": "Content of app.tar.gz.sig",
//!          "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-x86_64.app.tar.gz",
//!          "format": "app"
//!        },
//!        "macos-aarch64": {
//!          "signature": "Content of app.tar.gz.sig",
//!          "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-aarch64.app.tar.gz",
//!          "format": "app"
//!        },
//!        "linux-x86_64": {
//!          "signature": "Content of app.AppImage.sig",
//!          "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-amd64.AppImage.tar.gz",
//!          "format": "appimage"
//!        },
//!        "windows-x86_64": {
//!          "signature": "Content of app-setup.exe.sig or app.msi.sig, depending on the chosen format",
//!          "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-x64-setup.nsis.zip",
//!          "format": "nsis or wix depending on the chosen format"
//!        }
//!      }
//!    }
//!    ```
//!
//!  - **JSON for one platform**
//!
//!    ```json
//!    {
//!      "version": "0.2.0",
//!      "pub_date": "2020-09-18T12:29:53+01:00",
//!      "url": "https://mycompany.example.com/myapp/releases/myrelease.tar.gz",
//!      "signature": "Content of the relevant .sig file",
//!      "format": "app or nsis or wix or appimage depending on the release target and the chosen format",
//!      "notes": "These are some release notes"
//!    }
//!    ```
//!
//!
//! ## Update install mode on Windows
//!
//! You can specify which install mode to use on Windows using [`WindowsConfig::install_mode`] which can be one of:
//!
//! - [`"Passive"`](WindowsUpdateInstallMode::Passive): There will be a small window with a progress bar. The update will be installed without requiring any user interaction. Generally recommended and the default mode.
//! - [`"BasicUi"`](WindowsUpdateInstallMode::BasicUi): There will be a basic user interface shown which requires user interaction to finish the installation.
//! - [`"Quiet"`](WindowsUpdateInstallMode::Quiet): There will be no progress feedback to the user. With this mode the installer cannot request admin privileges by itself so it only works in user-wide installations or when your app itself already runs with admin privileges. Generally not recommended.

#![deny(missing_docs)]

use base64::Engine;
use cargo_packager_utils::current_exe::current_exe;
use http::{
    header::{ACCEPT, USER_AGENT},
    HeaderName,
};
use minisign_verify::{PublicKey, Signature};
use percent_encoding::{AsciiSet, CONTROLS};
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

mod custom_serialization;
mod error;

pub use crate::error::*;
pub use http;
pub use reqwest;
pub use semver;
pub use url;

/// Install modes for the Windows update.
#[derive(Debug, PartialEq, Eq, Clone, Default, Deserialize, Serialize)]
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
#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsConfig {
    /// Additional arguments given to the NSIS or WiX installer.
    pub installer_args: Option<Vec<String>>,
    /// The installation mode for the update on Windows. Defaults to `passive`.
    pub install_mode: Option<WindowsUpdateInstallMode>,
}

/// Updater configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// The updater endpoints.
    ///
    /// Each endpoint optionally could have `{{arch}}`, `{{target}}` or `{{current_version}}`
    /// which will be detected and replaced with the appropriate value before making a request to the endpoint.
    ///
    /// - `{{current_version}}`: The version of the app that is requesting the update.
    /// - `{{target}}`: The operating system name (one of `linux`, `windows` or `macos`).
    /// - `{{arch}}`: The architecture of the machine (one of `x86_64`, `i686`, `aarch64` or `armv7`).
    pub endpoints: Vec<Url>,
    /// Signature public key.
    pub pubkey: String,
    /// The Windows configuration for the updater.
    pub windows: Option<WindowsConfig>,
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

impl std::fmt::Display for UpdateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UpdateFormat::Nsis => "nsis",
                UpdateFormat::Wix => "wix",
                UpdateFormat::AppImage => "appimage",
                UpdateFormat::App => "app",
            }
        )
    }
}

/// Information about a release
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReleaseManifestPlatform {
    /// Download URL for the platform
    pub url: Url,
    /// Signature for the platform
    pub signature: String,
    /// Update format
    pub format: UpdateFormat,
}

/// Information about a release data.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RemoteReleaseData {
    /// Dynamic release data based on the platform the update has been requested from.
    Dynamic(ReleaseManifestPlatform),
    /// A map of release data for each platform, where the key is `<platform>-<arch>`.
    Static {
        /// A map of release data for each platform, where the key is `<platform>-<arch>`.
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
    pub data: RemoteReleaseData,
}

impl RemoteRelease {
    /// The release's download URL for the given target.
    pub fn download_url(&self, target: &str) -> Result<&Url> {
        match self.data {
            RemoteReleaseData::Dynamic(ref platform) => Ok(&platform.url),
            RemoteReleaseData::Static { ref platforms } => platforms
                .get(target)
                .map_or(Err(Error::TargetNotFound(target.to_string())), |p| {
                    Ok(&p.url)
                }),
        }
    }

    /// The release's signature for the given target.
    pub fn signature(&self, target: &str) -> Result<&String> {
        match self.data {
            RemoteReleaseData::Dynamic(ref platform) => Ok(&platform.signature),
            RemoteReleaseData::Static { ref platforms } => platforms
                .get(target)
                .map_or(Err(Error::TargetNotFound(target.to_string())), |platform| {
                    Ok(&platform.signature)
                }),
        }
    }

    /// The release's update format for the given target.
    pub fn format(&self, target: &str) -> Result<UpdateFormat> {
        match self.data {
            RemoteReleaseData::Dynamic(ref platform) => Ok(platform.format),
            RemoteReleaseData::Static { ref platforms } => platforms
                .get(target)
                .map_or(Err(Error::TargetNotFound(target.to_string())), |platform| {
                    Ok(platform.format)
                }),
        }
    }
}

/// An [`Updater`] builder.
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
    /// Create a new updater builder request.
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

    /// A custom function to compare whether a new version exists or not.
    pub fn version_comparator<F: Fn(Version, RemoteRelease) -> bool + Send + Sync + 'static>(
        mut self,
        f: F,
    ) -> Self {
        self.version_comparator = Some(Box::new(f));
        self
    }

    /// Specify a public key to use when checking if the update is valid.
    pub fn pub_key(mut self, pub_key: impl Into<String>) -> Self {
        self.config.pubkey = pub_key.into();
        self
    }

    /// Specify the target to request an update for.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target.replace(target.into());
        self
    }

    /// Specify the endpoints where an update will be requested from.
    pub fn endpoints(mut self, endpoints: Vec<Url>) -> Self {
        self.config.endpoints = endpoints;
        self
    }

    /// Specify the path to the current executable where the updater will try to update in the same directory.
    pub fn executable_path<P: AsRef<Path>>(mut self, p: P) -> Self {
        self.executable_path.replace(p.as_ref().into());
        self
    }

    /// Add a header to the updater request.
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

    /// Specify a timeout for the updater request.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Specify custom installer args on Windows.
    pub fn installer_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if self.config.windows.is_none() {
            self.config.windows.replace(Default::default());
        }
        self.config
            .windows
            .as_mut()
            .unwrap()
            .installer_args
            .replace(args.into_iter().map(Into::into).collect());
        self
    }

    /// Build the updater.
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

        let executable_path = match self.executable_path {
            Some(p) => p,
            #[cfg(not(any(windows, target_os = "macos")))]
            None => {
                if let Some(appimage) = std::env::var_os("APPIMAGE").map(PathBuf::from) {
                    appimage
                } else {
                    current_exe()?
                }
            }
            #[cfg(any(windows, target_os = "macos"))]
            _ => current_exe()?,
        };

        // Get the extract_path from the provided executable_path
        #[cfg(any(windows, target_os = "macos"))]
        let extract_path = extract_path_from_executable(&executable_path)?;
        #[cfg(not(any(windows, target_os = "macos")))]
        let extract_path = executable_path;

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

/// A type that can check for updates and created by [`UpdaterBuilder`].
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
    /// Check for an update. Returns `None` if an update was not found, otherwise it will be `Some`.
    pub fn check(&self) -> Result<Option<Update>> {
        // we want JSON only
        let mut headers = self.headers.clone();
        if !headers.contains_key(ACCEPT) {
            headers.insert(ACCEPT, HeaderValue::from_str("application/json").unwrap());
        }

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

        let version = self.current_version.to_string();
        let version = version.as_bytes();
        const CONTROLS_ADD: &AsciiSet = &CONTROLS.add(b'+');
        let encoded_version = percent_encoding::percent_encode(version, CONTROLS_ADD);
        let encoded_version = encoded_version.to_string();

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
                // url::Url automatically url-encodes the path components
                .replace("%7B%7Bcurrent_version%7D%7D", &encoded_version)
                .replace("%7B%7Btarget%7D%7D", &self.target)
                .replace("%7B%7Barch%7D%7D", self.arch)
                // but not query parameters
                .replace("{{current_version}}", &encoded_version)
                .replace("{{target}}", &self.target)
                .replace("{{arch}}", self.arch)
                .parse()?;

            log::debug!("checking for updates {url}");

            let mut request = Client::new().get(url).headers(headers.clone());
            if let Some(timeout) = self.timeout {
                request = request.timeout(timeout);
            }
            let response = request.send();

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        // no updates found!
                        if StatusCode::NO_CONTENT == res.status() {
                            log::debug!("update endpoint returned 204 No Content");
                            return Ok(None);
                        };

                        let update_response: serde_json::Value = res.json()?;
                        log::debug!("update response: {update_response:?}");

                        match serde_json::from_value::<RemoteRelease>(update_response)
                            .map_err(Into::into)
                        {
                            Ok(release) => {
                                log::debug!("parsed release response {release:?}");
                                last_error = None;
                                remote_release = Some(release);
                                // we found a relase, break the loop
                                break;
                            }
                            Err(err) => {
                                log::error!("failed to deserialize update response: {err}");
                                last_error = Some(err)
                            }
                        }
                    } else {
                        log::error!(
                            "update endpoint did not respond with a successful status code"
                        );
                    }
                }
                Err(err) => {
                    log::error!("failed to check for updates: {err}");
                    last_error = Some(err.into());
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

/// Information about an update and associted methods to perform the update.
#[derive(Debug, Clone)]
pub struct Update {
    /// Config used to check for this update.
    pub config: Config,
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
    pub extract_path: PathBuf,
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
    pub fn download(&self) -> Result<Vec<u8>> {
        self.download_extended_inner(
            None::<Box<dyn Fn(usize, Option<u64>)>>,
            None::<Box<dyn FnOnce()>>,
        )
    }

    /// Downloads the updater package, verifies it then return it as bytes.
    ///
    /// Takes two callbacks, the first will be excuted when receiveing each chunk
    /// while the second will be called only once when the download finishes.
    ///
    /// Use [`Update::install`] to install it
    pub fn download_extended<C: Fn(usize, Option<u64>), D: FnOnce()>(
        &self,
        on_chunk: C,
        on_download_finish: D,
    ) -> Result<Vec<u8>> {
        self.download_extended_inner(Some(on_chunk), Some(on_download_finish))
    }

    fn download_extended_inner<C: Fn(usize, Option<u64>), D: FnOnce()>(
        &self,
        on_chunk: Option<C>,
        on_download_finish: Option<D>,
    ) -> Result<Vec<u8>> {
        // set our headers
        let mut headers = self.headers.clone();
        if !headers.contains_key(ACCEPT) {
            headers.insert(
                ACCEPT,
                HeaderValue::from_str("application/octet-stream").unwrap(),
            );
        }
        if !headers.contains_key(USER_AGENT) {
            headers.insert(
                USER_AGENT,
                HeaderValue::from_str("cargo-packager-updater").unwrap(),
            );
        }

        let mut request = Client::new()
            .get(self.download_url.clone())
            .headers(headers);
        if let Some(timeout) = self.timeout {
            request = request.timeout(timeout);
        }

        struct DownloadProgress<R, C: Fn(usize, Option<u64>)> {
            content_length: Option<u64>,
            inner: R,
            on_chunk: Option<C>,
        }

        impl<R: Read, C: Fn(usize, Option<u64>)> Read for DownloadProgress<R, C> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.inner.read(buf).inspect(|&n| {
                    if let Some(on_chunk) = &self.on_chunk {
                        (on_chunk)(n, self.content_length);
                    }
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
        if let Some(on_download_finish) = on_download_finish {
            on_download_finish();
        }

        let mut update_buffer = Cursor::new(&buffer);

        verify_signature(&mut update_buffer, &self.signature, &self.config.pubkey)?;

        Ok(buffer)
    }

    /// Installs the updater package downloaded by [`Update::download`]
    pub fn install(&self, bytes: Vec<u8>) -> Result<()> {
        self.install_inner(bytes)
    }

    /// Downloads and installs the updater package
    pub fn download_and_install(&self) -> Result<()> {
        let bytes = self.download()?;
        self.install(bytes)
    }

    /// Downloads and installs the updater package
    ///
    /// Takes two callbacks, the first will be excuted when receiveing each chunk
    /// while the second will be called only once when the download finishes.
    pub fn download_and_install_extended<C: Fn(usize, Option<u64>), D: FnOnce()>(
        &self,
        on_chunk: C,
        on_download_finish: D,
    ) -> Result<()> {
        let bytes = self.download_extended(on_chunk, on_download_finish)?;
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
        use std::{io::Write, os::windows::process::CommandExt, process::Command};

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

        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // we support 2 type of files exe & msi for now
        // If it's an `exe` we expect an installer not a runtime.
        match self.format {
            UpdateFormat::Nsis => {
                // we need to wrap the installer path in quotes for Start-Process
                let mut installer_path = std::ffi::OsString::new();
                installer_path.push("\"");
                installer_path.push(&path);
                installer_path.push("\"");

                let installer_args = self
                    .config
                    .windows
                    .as_ref()
                    .and_then(|w| w.installer_args.clone())
                    .unwrap_or_default();
                let installer_args = [
                    self.config
                        .windows
                        .as_ref()
                        .and_then(|w| w.install_mode.clone())
                        .unwrap_or_default()
                        .nsis_args(),
                    installer_args
                        .iter()
                        .map(AsRef::as_ref)
                        .collect::<Vec<_>>()
                        .as_slice(),
                ]
                .concat();

                // Run the installer
                let mut cmd = Command::new(powershell_path);
                cmd.creation_flags(CREATE_NO_WINDOW)
                    .args(["-NoProfile", "-WindowStyle", "Hidden"])
                    .args(["Start-Process"])
                    .arg(installer_path);
                if !installer_args.is_empty() {
                    cmd.arg("-ArgumentList").arg(installer_args.join(", "));
                }
                cmd.spawn().expect("installer failed to start");

                std::process::exit(0);
            }
            UpdateFormat::Wix => {
                {
                    // we need to wrap the current exe path in quotes for Start-Process
                    let mut current_exe_arg = std::ffi::OsString::new();
                    current_exe_arg.push("\"");
                    current_exe_arg.push(current_exe()?);
                    current_exe_arg.push("\"");

                    let mut mis_path = std::ffi::OsString::new();
                    mis_path.push("\"\"\"");
                    mis_path.push(&path);
                    mis_path.push("\"\"\"");

                    let installer_args = self
                        .config
                        .windows
                        .as_ref()
                        .and_then(|w| w.installer_args.clone())
                        .unwrap_or_default();
                    let installer_args = [
                        self.config
                            .windows
                            .as_ref()
                            .and_then(|w| w.install_mode.clone())
                            .unwrap_or_default()
                            .msiexec_args(),
                        installer_args
                            .iter()
                            .map(AsRef::as_ref)
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ]
                    .concat();

                    // run the installer and relaunch the application
                    let powershell_install_res = Command::new(powershell_path)
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["-NoProfile", "-WindowStyle", "Hidden"])
                        .args([
                            "Start-Process",
                            "-Wait",
                            "-FilePath",
                            "$env:SYSTEMROOT\\System32\\msiexec.exe",
                            "-ArgumentList",
                        ])
                        .arg("/i,")
                        .arg(&mis_path)
                        .arg(format!(", {}, /promptrestart;", installer_args.join(", ")))
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
                            .arg(mis_path)
                            .args(installer_args)
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
        use std::fs;

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
            if let Some(tmp_dir_root) = tmp_dir_location() {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};

                let tmp_dir = tempfile::Builder::new()
                    .prefix("current_app")
                    .tempdir_in(tmp_dir_root)?;
                let tmp_dir_metadata = tmp_dir.path().metadata()?;

                if extract_path_metadata.dev() == tmp_dir_metadata.dev() {
                    let mut perms = tmp_dir_metadata.permissions();
                    perms.set_mode(0o700);
                    fs::set_permissions(&tmp_dir, perms)?;

                    let tmp_app_image = tmp_dir.path().join("current_app.AppImage");

                    // get metadata to restore later
                    let metadata = self.extract_path.metadata()?;

                    // create a backup of our current app image
                    fs::rename(&self.extract_path, &tmp_app_image)?;

                    // if something went wrong during the extraction, we should restore previous app
                    if let Err(err) = fs::write(&self.extract_path, bytes).and_then(|_| {
                        fs::set_permissions(&self.extract_path, metadata.permissions())
                    }) {
                        fs::rename(tmp_app_image, &self.extract_path)?;
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

        // Create temp directories for backup and extraction
        let tmp_backup_dir = tempfile::Builder::new()
            .prefix("packager_current_app")
            .tempdir()?;

        let tmp_extract_dir = tempfile::Builder::new()
            .prefix("packager_updated_app")
            .tempdir()?;

        let decoder = GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);

        // Extract files to temporary directory
        for entry in archive.entries()? {
            let mut entry = entry?;
            let collected_path: PathBuf = entry.path()?.iter().skip(1).collect();
            let extraction_path = tmp_extract_dir.path().join(&collected_path);

            // Ensure parent directories exist
            if let Some(parent) = extraction_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if let Err(err) = entry.unpack(&extraction_path) {
                // Cleanup on error
                std::fs::remove_dir_all(tmp_extract_dir.path()).ok();
                return Err(err.into());
            }
            extracted_files.push(extraction_path);
        }

        // Try to move the current app to backup
        let move_result = std::fs::rename(
            &self.extract_path,
            tmp_backup_dir.path().join("current_app"),
        );
        let need_authorization = if let Err(err) = move_result {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                true
            } else {
                std::fs::remove_dir_all(tmp_extract_dir.path()).ok();
                return Err(err.into());
            }
        } else {
            false
        };

        if need_authorization {
            log::debug!("app installation needs admin privileges");
            // Use AppleScript to perform moves with admin privileges
            let apple_script = format!(
                "do shell script \"rm -rf '{src}' && mv -f '{new}' '{src}'\" with administrator privileges",
                src = self.extract_path.display(),
                new = tmp_extract_dir.path().display()
            );

            let res = std::process::Command::new("osascript")
                .arg("-e")
                .arg(apple_script)
                .status();

            if res.is_err() || !res.unwrap().success() {
                log::error!("failed to install update using AppleScript");
                std::fs::remove_dir_all(tmp_extract_dir.path()).ok();
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Failed to move the new app into place",
                )));
            }
        } else {
            // Remove existing directory if it exists
            if self.extract_path.exists() {
                std::fs::remove_dir_all(&self.extract_path)?;
            }
            // Move the new app to the target path
            std::fs::rename(tmp_extract_dir.path(), &self.extract_path)?;
        }

        let _ = std::process::Command::new("touch")
            .arg(&self.extract_path)
            .status();

        Ok(())
    }
}

/// Check for an update using the provided
pub fn check_update(current_version: Version, config: crate::Config) -> Result<Option<Update>> {
    UpdaterBuilder::new(current_version, config)
        .build()?
        .check()
}

/// Get the updater target for the current platform.
#[doc(hidden)]
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
    } else if cfg!(target_arch = "riscv64") {
        Some("riscv64")
    } else {
        None
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn extract_path_from_executable(executable_path: &Path) -> Result<PathBuf> {
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
fn verify_signature<R>(
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
