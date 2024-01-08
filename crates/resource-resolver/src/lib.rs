// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! # cargo-packager-updater
//!
//! Resource resolver for apps that were packaged by [`cargo-packager`](https://docs.rs/cargo-packager).
//!
//! It resolves the root path which contains resources, which was set using the `resources`
//! field of [cargo packager configuration](https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html).
//!
//! ## Get the resource path
//!
//! ```
//! use cargo_packager_resource_resolver::{resources_dir, PackageFormat};
//!
//! let resource_path = resources_dir(PackageFormat::Nsis).unwrap();
//! ```
//! ## Automatically detect formats
//!
//! <div class="warning">
//!
//! This feature is only available for apps that were built with cargo packager. So the node js binding will not work.
//!
//! </div>
//!
//! 1. Make sure to use the `before_each_package_command` field of [cargo packager configuration](https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html) to build your app (this will not work with the `before_packaging_command` field).
//! 2. Active the feature `auto-detect-format`.
//!
//! ```rs
//! use cargo_packager_resource_resolver::{resources_dir, current_format};
//!
//! let resource_path = resources_dir(current_format()).unwrap();
//! ```
//!
use error::Result;
use std::{env, path::PathBuf};

mod error;

pub use error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageFormat {
    /// When no format is used (`cargo run`)
    None,
    /// The macOS application bundle (.app).
    App,
    /// The macOS DMG package (.dmg).
    Dmg,
    /// The Microsoft Software Installer (.msi) through WiX Toolset.
    Wix,
    /// The NSIS installer (.exe).
    Nsis,
    /// The Linux Debian package (.deb).
    Deb,
    /// The Linux AppImage package (.AppImage).
    AppImage,
}

/// Get the current package format.
/// Can only be used if the app was build with cargo-packager
/// and when the `before-each-package-command` Cargo feature is enabled.
#[cfg(feature = "auto-detect-format")]
#[must_use]
pub fn current_format() -> PackageFormat {
    // sync with PackageFormat::short_name function of packager crate
    // maybe having a special crate for the Config struct,
    // that both packager and resource-resolver could be a
    // better alternative
    if cfg!(CARGO_PACKAGER_FORMAT = "app") {
        PackageFormat::App
    } else if cfg!(CARGO_PACKAGER_FORMAT = "dmg") {
        PackageFormat::Dmg
    } else if cfg!(CARGO_PACKAGER_FORMAT = "wix") {
        PackageFormat::Wix
    } else if cfg!(CARGO_PACKAGER_FORMAT = "nsis") {
        PackageFormat::Nsis
    } else if cfg!(CARGO_PACKAGER_FORMAT = "deb") {
        PackageFormat::Deb
    } else if cfg!(CARGO_PACKAGER_FORMAT = "appimage") {
        PackageFormat::AppImage
    } else {
        PackageFormat::None
    }
}

/// Retrieve the resource path of your app, packaged with cargo packager.
///
/// ## Example
///
/// ```
/// use cargo_packager_resource_resolver::{resources_dir, PackageFormat};
///
/// let resource_path = resources_dir(PackageFormat::Nsis).unwrap();
/// ```
///
pub fn resources_dir(package_format: PackageFormat) -> Result<PathBuf> {
    match package_format {
        PackageFormat::None => {
            env::current_dir().map_err(|e| Error::Io("Can't access current dir".to_string(), e))
        }
        PackageFormat::App | PackageFormat::Dmg => {
            let exe = current_exe()?;
            let exe_dir = exe.parent().unwrap();
            exe_dir
                .join("../Resources")
                .canonicalize()
                .map_err(|e| Error::Io("".to_string(), e))
        }
        PackageFormat::Wix | PackageFormat::Nsis => {
            let exe = current_exe()?;
            let exe_dir = exe.parent().unwrap();
            Ok(exe_dir.to_path_buf())
        }
        PackageFormat::Deb | PackageFormat::AppImage => {
            let exe = current_exe()?;
            let binary_name = exe.file_name().unwrap().to_string_lossy();

            let path = format!("/usr/lib/{}/", binary_name);
            Ok(PathBuf::from(path))
        }
    }
}

fn current_exe() -> Result<PathBuf> {
    cargo_packager_utils::current_exe::current_exe()
        .map_err(|e| Error::Io("Can't detect the path of the current exe".to_string(), e))
}
