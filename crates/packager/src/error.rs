use std::path::PathBuf;

use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
/// Errors returned by cargo-packager.
pub enum Error {
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
    /// I/O errors.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Hex de/encoding errors.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Failed to validate downloaded file hash.
    #[error("hash mismatch of downloaded file")]
    HashError,
    /// Zip error.
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
    /// Zip error.
    #[error(transparent)]
    DownloadError(#[from] Box<ureq::Error>),
    /// Unsupported OS bitness.
    #[error("unsupported OS bitness")]
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
    #[error("Couldn't find the main binary in list of provided binaries")]
    MainBinaryNotFound,
    /// Semver parsing error
    #[error(transparent)]
    Semver(#[from] semver::Error),
    /// Non-numeric build metadata in app version.
    #[error("optional build metadata in app version must be numeric-only {}", .0.clone().unwrap_or_default())]
    NonNumericBuildMetadata(Option<String>),
    /// Invalid app version when building [crate::PackageFormat::Msi]
    #[error("invalid app version: {0}")]
    InvalidAppVersion(String),
    /// Handlebars render error.
    #[error(transparent)]
    HandleBarsRenderError(#[from] handlebars::RenderError),
    /// Handlebars template error.
    #[error(transparent)]
    HandleBarsTemplateError(#[from] Box<handlebars::TemplateError>),
    /// Nsis error
    #[error("error running makensis.exe: {0}")]
    NsisFailed(String),
    /// Nsis error
    #[error("error running {0}: {0}")]
    WixFailed(String, String),
    /// Failed to get parent directory of a path
    #[error("Failed to get parent directory of a path")]
    ParentDirNotFound,
    #[error("{0} `{1}` failed with exit code {2}")]
    HookCommandFailure(String, String, i32),
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
    #[error("Language {0} not found. It must be one of {1}")]
    UnsupportedWixLanguage(String, String),
    /// image crate errors.
    #[error(transparent)]
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    ImageError(#[from] image::ImageError),
    /// walkdir crate errors.
    #[error(transparent)]
    WalkDirError(#[from] walkdir::Error),
    /// Path prefix strip error.
    #[error(transparent)]
    StripPrefixError(#[from] std::path::StripPrefixError),
    /// std::process::Command program failed
    #[error("Command failed")]
    CommandFailed,
    /// Relative paths errors
    #[error(transparent)]
    RelativeToError(#[from] relative_path::RelativeToError),
    /// Time error.
    #[cfg(target_os = "macos")]
    #[error("`{0}`")]
    TimeError(#[from] time::error::Error),
    /// Plist error.
    #[cfg(target_os = "macos")]
    #[error(transparent)]
    Plist(#[from] plist::Error),
    /// Framework not found.
    #[cfg(target_os = "macos")]
    #[error("framework {0} not found")]
    FrameworkNotFound(String),
    /// Invalid framework.
    #[cfg(target_os = "macos")]
    #[error("invalid framework {framework}: {reason}")]
    InvalidFramework {
        framework: String,
        reason: &'static str,
    },
    /// Image error.
    #[cfg(target_os = "macos")]
    #[error(transparent)]
    ImageError(#[from] image::ImageError),
    /// Invalid icons.
    #[cfg(target_os = "macos")]
    #[error("could not find a valid icon")]
    InvalidIconList,
    /// Failed to notarize.
    #[cfg(target_os = "macos")]
    #[error("failed to notarize app")]
    FailedToNotarize,
    /// Rejected on notarize.
    #[cfg(target_os = "macos")]
    #[error("failed to notarize app: {0}")]
    NotarizeRejected(String),
    /// Failed to parse notarytool output.
    #[cfg(target_os = "macos")]
    #[error("failed to parse notarytool output as JSON: `{0}`")]
    FailedToParseNotarytoolOutput(String),
    /// Failed to find API key file.
    #[cfg(target_os = "macos")]
    #[error("could not find API key file. Please set the APPLE_API_KEY_PATH environment variables to the path to the {filename} file")]
    ApiKeyMissing { filename: String },
    /// Missing notarize environment variables.
    #[cfg(target_os = "macos")]
    #[error("no APPLE_ID & APPLE_PASSWORD or APPLE_API_KEY & APPLE_API_ISSUER & APPLE_API_KEY_PATH environment variables found")]
    MissingNotarizeAuthVars,
    /// Path already exists.
    #[error("{0} already exists")]
    AlreadyExists(PathBuf),
    /// Path does not exist.
    #[error("{0} does not exist")]
    DoesNotExist(PathBuf),
    /// Path is not a directory.
    #[error("{0} is not a directory")]
    IsNotDirectory(PathBuf),
    /// Failed to run command.
    #[error("failed to run command {0}")]
    FailedToRunCommand(String),
    /// Couldn't find a square icon to use as AppImage icon
    #[error("couldn't find a square icon to use as AppImage icon")]
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
    #[error("Key generation aborted, {0} already exists and force overrwite wasnot desired.")]
    SigningKeyExists(PathBuf),
}

/// Convenient type alias of Result type for cargo-packager.
pub type Result<T> = std::result::Result<T, Error>;
