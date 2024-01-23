// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! # cargo-packager-resource-resolver
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
use std::path::PathBuf;

use cargo_packager_utils::current_exe::current_exe;
pub use cargo_packager_utils::PackageFormat;
use error::Result;

mod error;

pub use error::Error;

/// Get the current package format.
/// Can only be used if the app was build with cargo-packager
/// and when the `before-each-package-command` Cargo feature is enabled.
#[cfg(feature = "auto-detect-format")]
pub fn current_format() -> crate::Result<PackageFormat> {
    // sync with PackageFormat::short_name function of packager crate
    // maybe having a special crate for the Config struct,
    // that both packager and resource-resolver could be a
    // better alternative
    if cfg!(CARGO_PACKAGER_FORMAT = "app") {
        Ok(PackageFormat::App)
    } else if cfg!(CARGO_PACKAGER_FORMAT = "dmg") {
        Ok(PackageFormat::Dmg)
    } else if cfg!(CARGO_PACKAGER_FORMAT = "wix") {
        Ok(PackageFormat::Wix)
    } else if cfg!(CARGO_PACKAGER_FORMAT = "nsis") {
        Ok(PackageFormat::Nsis)
    } else if cfg!(CARGO_PACKAGER_FORMAT = "deb") {
        Ok(PackageFormat::Deb)
    } else if cfg!(CARGO_PACKAGER_FORMAT = "appimage") {
        Ok(PackageFormat::AppImage)
    } else {
        Err(Error::UnkownPackageFormat)
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
pub fn resources_dir(package_format: PackageFormat) -> Result<PathBuf> {
    match package_format {
        PackageFormat::App | PackageFormat::Dmg => {
            let exe = current_exe()?;
            let exe_dir = exe
                .parent()
                .ok_or_else(|| Error::ParentNotFound(exe.clone()))?;
            Ok(exe_dir.join("../Resources"))
        }
        PackageFormat::Wix | PackageFormat::Nsis => {
            let exe = current_exe()?;
            let exe_dir = exe
                .parent()
                .ok_or_else(|| Error::ParentNotFound(exe.clone()))?;
            Ok(exe_dir.to_path_buf())
        }
        PackageFormat::Deb => {
            let exe = current_exe()?;
            let exe_name = exe.file_name().unwrap().to_string_lossy();

            let path = format!("/usr/lib/{}/", exe_name);
            Ok(PathBuf::from(path))
        }

        PackageFormat::AppImage => {
            let appdir = std::env::var_os("APPDIR").ok_or(Error::AppDirNotFound)?;

            // validate that we're actually running on an AppImage
            // an AppImage is mounted to `/$TEMPDIR/.mount_${appPrefix}${hash}`
            // see https://github.com/AppImage/AppImageKit/blob/1681fd84dbe09c7d9b22e13cdb16ea601aa0ec47/src/runtime.c#L501
            // note that it is safe to use `std::env::current_exe` here since we just loaded an AppImage.
            let is_temp = std::env::current_exe()
                .map(|p| {
                    p.display()
                        .to_string()
                        .starts_with(&format!("{}/.mount_", std::env::temp_dir().display()))
                })
                .unwrap_or(true);

            if !is_temp {
                return Err(Error::InvalidAppImage);
            }

            let appdir: &std::path::Path = appdir.as_ref();

            let exe = current_exe()?;
            let exe_name = exe.file_name().unwrap().to_string_lossy();

            Ok(PathBuf::from(format!(
                "{}/usr/lib/{}",
                appdir.display(),
                exe_name
            )))
        }
        _ => Err(Error::UnsupportedPackageFormat),
    }
}
