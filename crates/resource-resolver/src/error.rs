// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

/// The result type of `resource-resolver`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type of `resource-resolver`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// IO error
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    /// Environment error
    #[error("{0}")]
    Env(String),
    /// Environment variable error
    #[error("{0}: {1}")]
    Var(String, std::env::VarError),
}
