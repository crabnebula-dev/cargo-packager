//! cargo-packager is a tool that generates installers or app bundles for rust executables.
//! It supports auto updating through [cargo-update-packager](https://docs.rs/cargo-update-packager).
//!
//! # Platform support
//! - macOS
//!   - DMG (.dmg)
//!   - Bundle (.app)
//! - Linux
//!   - Debian package (.deb)
//!   - AppImage (.AppImage)
//! - Windows
//!   - MSI using WiX Toolset (.msi)
//!   - NSIS (.exe)
#![cfg_attr(doc_cfg, feature(doc_cfg))]

use std::path::PathBuf;

mod codesign;
mod error;
mod package;
mod shell;
mod util;

#[cfg(feature = "cli")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "cli")))]
pub mod cli;
pub mod config;
pub mod sign;

pub use config::{Config, PackageFormat};
pub use error::{Error, Result};
pub use sign::SigningConfig;

pub use package::{package, PackageOuput};

/// Sign the specified packages and return the signatures paths.
pub fn sign_outputs(
    config: &SigningConfig,
    packages: &[PackageOuput],
) -> crate::Result<Vec<PathBuf>> {
    let mut signatures = Vec::new();
    for package in packages {
        if !matches!(package.format, PackageFormat::App) {
            for path in &package.paths {
                signatures.push(sign::sign_file(config, path)?);
            }
        }
    }

    Ok(signatures)
}

/// Package an app using the specified config.
/// Then signs the generated packages.
///
/// This is similar to calling `sign_outputs(signing_config, package(config)?)`
///
/// Returns a tuple of list of packages and list of signatures.
pub fn package_and_sign(
    config: &Config,
    signing_config: &SigningConfig,
) -> crate::Result<(Vec<PackageOuput>, Vec<PathBuf>)> {
    let packages = package(config)?;
    let signatures = sign_outputs(signing_config, &packages)?;
    Ok((packages, signatures))
}
