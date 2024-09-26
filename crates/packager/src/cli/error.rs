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
    #[cfg(feature = "cli")]
    Clap(#[from] clap::error::Error),
    /// Error while reading cargo metadata.
    #[error("Failed to read cargo metadata: {0}")]
    Metadata(#[from] cargo_metadata::Error),
    /// JSON parsing error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// TOML parsing error.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    /// JSON Config parsing error.
    #[error("Failed to parse config: {0}")]
    FailedToParseJsonConfig(serde_json::Error),
    #[error("Failed to deserialize config from `package.metadata.packager` in Cargo.toml: {0}")]
    FailedToParseJsonConfigCargoToml(serde_json::Error),
    /// TOML Config parsing error.
    #[error("Failed to parse config: {0}")]
    FailedToParseTomlConfig(Box<toml::de::Error>),
    /// Cargo.toml parsing error.
    #[error("Failed to parse Cargo.toml: {0}")]
    FailedToParseCargoToml(Box<toml::de::Error>),
    /// package.json parsing error.
    #[error("Failed to parse package.json: {0}")]
    FailedToParsePacakgeJson(serde_json::Error),
    /// JSON Config parsing error.
    #[error("Failed to parse config at {0}: {1}")]
    FailedToParseJsonConfigFromPath(PathBuf, serde_json::Error),
    /// TOML Config parsing error.
    #[error("Failed to parse config at {0}: {1}")]
    FailedToParseTomlConfigFromPath(PathBuf, Box<toml::de::Error>),
    /// I/O errors with path.
    #[error("I/O Error ({0}): {1}")]
    IoWithPath(PathBuf, std::io::Error),
    /// I/O errors.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Packaging error
    #[error(transparent)]
    Packaging(#[from] crate::Error),
}

/// Convenient type alias of Result type for cargo-packager.
pub type Result<T> = std::result::Result<T, Error>;
