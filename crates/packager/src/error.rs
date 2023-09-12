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
}

/// Convenient type alias of Result type for cargo-packager.
pub type Result<T> = std::result::Result<T, Error>;
