/// The result type of `resource-resolver`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type of `resource-resolver`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Tried to get resource on an unsupported platform
    #[error("Unsupported platform for reading resources")]
    UnsupportedPlatform,
    /// IO error
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    /// Environement error
    #[error("{0}")]
    Env(String),
    /// Environement variable error
    #[error("{0}: {1}")]
    Var(String, std::env::VarError),
}
