// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
/// Errors returned by cargo-packager.
pub enum Error {
    /// Clap error.
    #[error(transparent)]
    Clap(#[from] clap::error::Error),
    /// Error while reading cargo metadata.
    #[error("Failed to read cargo metadata: {0}")]
    Metadata(#[from] cargo_metadata::Error),
    /// JSON Config parsing error.
    #[error("Failed to parse config: {0}")]
    JSONConfigParseError(#[from] serde_json::Error),
    /// TOML Config parsing error.
    #[error("Failed to parse config: {0}")]
    TOMLConfigParseError(#[from] toml::de::Error),
    /// Target triple architecture error
    #[error("Unable to determine target-architecture")]
    Architecture,
    /// Target triple OS error
    #[error("Unable to determine target-os")]
    Os,
    /// Target triple environment error
    #[error("Unable to determine target-environment")]
    Environment,
    /// No config file found.
    #[error("Couldn't detect a valid configuration file or all configurations are disabled.")]
    NoConfig,
    /// I/O errors.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Hex de/encoding errors.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Failed to validate downloaded file hash.
    #[error("Hash mismatch of downloaded file")]
    HashError,
    /// Zip error.
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
    /// Zip error.
    #[error(transparent)]
    DownloadError(#[from] Box<ureq::Error>),
    /// Unsupported OS bitness.
    #[error("Unsupported OS bitness")]
    UnsupportedBitness,
    /// Windows SignTool not found.
    #[error("SignTool not found")]
    SignToolNotFound,
    /// Unexpected target triple.
    #[error("Unexpected target triple: {0}")]
    UnexpectedTargetTriple(String),
    /// Unsupported architecture.
    #[error("Unsupported architecture for \"{0}\" target triple: {0}")]
    UnsupportedArch(String, String),
    /// Could not find the main binary in list of provided binaries.
    #[error("Could not find the main binary in list of provided binaries")]
    MainBinaryNotFound,
    /// Semver parsing error
    #[error(transparent)]
    Semver(#[from] semver::Error),
    /// Non-numeric build metadata in app version.
    #[error("Optional build metadata in app version must be numeric-only {}", .0.clone().unwrap_or_default())]
    NonNumericBuildMetadata(Option<String>),
    /// Invalid app version when building [crate::PackageFormat::Wix]
    #[error("Invalid app version: {0}")]
    InvalidAppVersion(String),
    /// Handlebars render error.
    #[error(transparent)]
    HandleBarsRenderError(#[from] handlebars::RenderError),
    /// Handlebars template error.
    #[error(transparent)]
    HandleBarsTemplateError(#[from] Box<handlebars::TemplateError>),
    /// Nsis error
    #[error("Error running makensis.exe: {0}")]
    NsisFailed(std::io::Error),
    /// Nsis error
    #[error("Error running {0}: {0}")]
    WixFailed(String, std::io::Error),
    /// create-dmg script error
    #[error("Error running create-dmg script: {0}")]
    CreateDmgFailed(std::io::Error),
    /// signtool.exe error
    #[error("Error running signtool.exe: {0}")]
    SignToolFailed(std::io::Error),
    /// Custom signing command error
    #[error("Error running custom signing command: {0}")]
    CustomSignCommandFailed(std::io::Error),
    /// bundle_appimage script error
    #[error("Error running bundle_appimage.sh script: {0}")]
    AppImageScriptFailed(std::io::Error),
    /// Failed to get parent directory of a path
    #[error("Failed to get parent directory of {0}")]
    ParentDirNotFound(std::path::PathBuf),
    /// A hook, for example `beforePackagaingCommand`, has failed.
    #[error("{0} `{1}` failed: {2}")]
    HookCommandFailure(String, String, std::io::Error),
    /// A hook, for example `beforePackagaingCommand`, has failed with an exit code.
    #[error("{0} `{1}` failed with exit code {2}")]
    HookCommandFailureWithExitCode(String, String, i32),
    /// Regex error.
    #[cfg(windows)]
    #[error(transparent)]
    RegexError(#[from] regex::Error),
    /// Glob pattern error.
    #[error(transparent)]
    GlobPatternError(#[from] glob::PatternError),
    /// Glob error.
    #[error(transparent)]
    Glob(#[from] glob::GlobError),
    /// Unsupported WiX language
    #[cfg(windows)]
    #[error("Wix language {0} not found. It must be one of {1}")]
    UnsupportedWixLanguage(String, String),
    /// Image crate errors.
    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    #[error(transparent)]
    ImageError(#[from] image::ImageError),
    /// walkdir crate errors.
    #[error(transparent)]
    WalkDirError(#[from] walkdir::Error),
    /// Path prefix strip error.
    #[error(transparent)]
    StripPrefixError(#[from] std::path::StripPrefixError),
    /// Relative paths errors
    #[error(transparent)]
    RelativeToError(#[from] relative_path::RelativeToError),
    /// Time error.
    #[error("`{0}`")]
    #[cfg(target_os = "macos")]
    TimeError(#[from] time::error::Error),
    /// Plist error.
    #[error(transparent)]
    #[cfg(target_os = "macos")]
    Plist(#[from] plist::Error),
    /// Framework not found.
    #[error("Framework {0} not found")]
    FrameworkNotFound(String),
    /// Invalid framework.
    #[error("Invalid framework {framework}: {reason}")]
    InvalidFramework {
        /// Framework name
        framework: String,
        /// Reason why this framework is invalid
        reason: &'static str,
    },
    /// Invalid icons.
    #[error("Could not find a valid icon")]
    InvalidIconList,
    /// Failed to notarize.
    #[error("Failed to notarize app")]
    FailedToNotarize,
    /// Rejected on notarize.
    #[error("Failed to notarize app: {0}")]
    NotarizeRejected(String),
    /// Failed to parse notarytool output.
    #[error("Failed to parse notarytool output as JSON: `{0}`")]
    FailedToParseNotarytoolOutput(String),
    /// Failed to find API key file.
    #[error("Could not find API key file. Please set the APPLE_API_KEY_PATH environment variables to the path to the {filename} file")]
    ApiKeyMissing {
        /// Filename of the API key.
        filename: String,
    },
    /// Missing notarize environment variables.
    #[error("Could not find APPLE_ID & APPLE_PASSWORD & APPLE_TEAM_ID or APPLE_API_KEY & APPLE_API_ISSUER & APPLE_API_KEY_PATH environment variables found")]
    MissingNotarizeAuthVars,
    /// Failed to list keychains
    #[error("Failed to list keychains: {0}")]
    FailedToListKeyChain(std::io::Error),
    /// Failed to decode certficate as base64
    #[error("Failed to decode certficate as base64: {0}")]
    FailedToDecodeCert(std::io::Error),
    /// Failed to create keychain.
    #[error("Failed to create keychain: {0}")]
    FailedToCreateKeyChain(std::io::Error),
    /// Failed to create keychain.
    #[error("Failed to unlock keychain: {0}")]
    FailedToUnlockKeyChain(std::io::Error),
    /// Failed to import certificate.
    #[error("Failed to import certificate: {0}")]
    FailedToImportCert(std::io::Error),
    /// Failed to set keychain settings.
    #[error("Failed to set keychain settings: {0}")]
    FailedToSetKeychainSettings(std::io::Error),
    /// Failed to set key partition list.
    #[error("Failed to set key partition list: {0}")]
    FailedToSetKeyPartitionList(std::io::Error),
    /// Failed to run codesign utility.
    #[error("Failed to run codesign utility: {0}")]
    FailedToRunCodesign(std::io::Error),
    /// Failed to run ditto utility.
    #[error("Failed to run ditto utility: {0}")]
    FailedToRunDitto(std::io::Error),
    /// Failed to run xcrun utility.
    #[error("Failed to run xcrun utility: {0}")]
    FailedToRunXcrun(std::io::Error),
    /// Path already exists.
    #[error("{0} already exists")]
    AlreadyExists(PathBuf),
    /// Path does not exist.
    #[error("{0} does not exist")]
    DoesNotExist(PathBuf),
    /// Path is not a directory.
    #[error("{0} is not a directory")]
    IsNotDirectory(PathBuf),
    /// Could not find a square icon to use as AppImage icon
    #[error("Could not find a square icon to use as AppImage icon")]
    AppImageSquareIcon,
    /// Base64 decoding error.
    #[error(transparent)]
    Base64DecodeError(#[from] base64::DecodeError),
    /// Utf8 parsing error.
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    /// minisign errors.
    #[error(transparent)]
    Minisign(#[from] minisign::PError),
    /// System time errors.
    #[error(transparent)]
    SystemTimeError(#[from] std::time::SystemTimeError),
    /// Signing keys generation error.
    #[error("aborted key generation, {0} already exists and force overrwite wasnot desired.")]
    SigningKeyExists(PathBuf),
    /// Failed to extract external binary filename
    #[error("Failed to extract filename from {0}")]
    FailedToExtractFilename(PathBuf),
    /// Failed to remove extended attributes from app bundle
    #[error("Failed to remove extended attributes from app bundle: {0}")]
    #[cfg(target_os = "macos")]
    FailedToRemoveExtendedAttributes(std::io::Error),
}

/// Convenient type alias of Result type for cargo-packager.
pub type Result<T> = std::result::Result<T, Error>;
