// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

/// The result type of `resource-resolver`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type of `resource-resolver`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Unkown package format.
    #[error("Unkown package format")]
    UnkownPackageFormat,
    /// Unsupported package format.
    #[error("Unsupported package format")]
    UnsupportedPackageFormat,
    /// Couldn't find `APPDIR` environment variable.
    #[error("Couldn't find `APPDIR` environment variable")]
    AppDirNotFound,
    /// `APPDIR` or `APPIMAGE` environment variable found but this application was not detected as an AppImage; this might be a security issue.
    #[error("`APPDIR` or `APPIMAGE` environment variable found but this application was not detected as an AppImage; this might be a security issue.")]
    InvalidAppImage,
    /// Couldn't find parent of path.
    #[error("Couldn't find parent of {0}")]
    ParentNotFound(PathBuf),
}
