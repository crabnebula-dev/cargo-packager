use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
/// Errors returned by cargo-packager.
pub enum Error {
    /// Error while reading cargo metadata.
    #[error("Failed to read cargo metadata: {0}")]
    Metadata(#[from] cargo_metadata::Error),
    /// Config parsing error.
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] serde_json::Error),
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
    DownloadError(#[from] ureq::Error),
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
    #[error("optional build metadata in app version must be numeric-only {}", .0.unwrap_or_default())]
    NonNumericBuildMetadata(Option<String>),
    /// Invalid app version when building [crate::PackageFormat::Msi]
    #[error("invalid app version: {0}")]
    InvalidAppVersion(String),
    /// Handlebars template error.
    #[error(transparent)]
    HandleBarsError(#[from] handlebars::RenderError),
    /// Nsis error
    #[error("error running makensis.exe: {0}")]
    NsisFailed(String),
    #[error("Failed to get parent directory of a path")]
    ParentDirNotFound,
    #[error("{0} `{1}` failed with exit code {2}")]
    HookCommandFailure(String, String, i32),
}

/// Convenient type alias of Result type for cargo-packager.
pub type Result<T> = std::result::Result<T, Error>;
