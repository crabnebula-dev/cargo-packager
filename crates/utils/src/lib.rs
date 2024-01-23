// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! # cargo-packager-utils
//!
//! Contain reusable components of the cargo-packager ecosystem.

use std::fmt::Display;

pub mod current_exe;

// NOTE: When making changes to this enum,
// make sure to also update in updater and resource-resolver bindings if needed
/// Types of supported packages by [`cargo-packager`](https://docs.rs/cargo-packager).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", value(rename_all = "lowercase"))]
#[non_exhaustive]
pub enum PackageFormat {
    /// All available package formats for the current platform.
    ///
    /// See [`PackageFormat::platform_all`]
    #[cfg(feature = "cli")]
    All,
    /// The default list of package formats for the current platform.
    ///
    /// See [`PackageFormat::platform_default`]
    #[cfg(feature = "cli")]
    Default,
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
    /// The Linux Pacman package (.tar.gz and PKGBUILD)
    Pacman,
}

impl Display for PackageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

impl PackageFormat {
    /// Maps a short name to a [PackageFormat].
    /// Possible values are "deb", "pacman", "appimage", "dmg", "app", "wix", "nsis".
    pub fn from_short_name(name: &str) -> Option<PackageFormat> {
        // Other types we may eventually want to support: apk.
        match name {
            "app" => Some(PackageFormat::App),
            "dmg" => Some(PackageFormat::Dmg),
            "wix" => Some(PackageFormat::Wix),
            "nsis" => Some(PackageFormat::Nsis),
            "deb" => Some(PackageFormat::Deb),
            "appimage" => Some(PackageFormat::AppImage),
            _ => None,
        }
    }

    /// Gets the short name of this [PackageFormat].
    pub fn short_name(&self) -> &'static str {
        match *self {
            #[cfg(feature = "cli")]
            PackageFormat::All => "all",
            #[cfg(feature = "cli")]
            PackageFormat::Default => "default",
            PackageFormat::App => "app",
            PackageFormat::Dmg => "dmg",
            PackageFormat::Wix => "wix",
            PackageFormat::Nsis => "nsis",
            PackageFormat::Deb => "deb",
            PackageFormat::AppImage => "appimage",
            PackageFormat::Pacman => "pacman",
        }
    }

    /// Gets the list of the possible package types on the current OS.
    ///
    /// - **macOS**: App, Dmg
    /// - **Windows**: Nsis, Wix
    /// - **Linux**: Deb, AppImage, Pacman
    pub fn platform_all() -> &'static [PackageFormat] {
        &[
            #[cfg(target_os = "macos")]
            PackageFormat::App,
            #[cfg(target_os = "macos")]
            PackageFormat::Dmg,
            #[cfg(target_os = "windows")]
            PackageFormat::Wix,
            #[cfg(target_os = "windows")]
            PackageFormat::Nsis,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Deb,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::AppImage,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Pacman,
        ]
    }

    /// Returns the default list of targets this platform
    ///
    /// - **macOS**: App, Dmg
    /// - **Windows**: Nsis
    /// - **Linux**: Deb, AppImage, Pacman
    pub fn platform_default() -> &'static [PackageFormat] {
        &[
            #[cfg(target_os = "macos")]
            PackageFormat::App,
            #[cfg(target_os = "macos")]
            PackageFormat::Dmg,
            #[cfg(target_os = "windows")]
            PackageFormat::Nsis,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Deb,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::AppImage,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Pacman,
        ]
    }

    /// Gets a number representing priority which used to sort package types
    /// in an order that guarantees that if a certain package type
    /// depends on another (like Dmg depending on MacOsBundle), the dependency
    /// will be built first
    ///
    /// The lower the number, the higher the priority
    pub fn priority(&self) -> u32 {
        match self {
            #[cfg(feature = "cli")]
            PackageFormat::All => 0,
            #[cfg(feature = "cli")]
            PackageFormat::Default => 0,
            PackageFormat::App => 0,
            PackageFormat::Wix => 0,
            PackageFormat::Nsis => 0,
            PackageFormat::Deb => 0,
            PackageFormat::AppImage => 0,
            PackageFormat::Pacman => 0,
            PackageFormat::Dmg => 1,
        }
    }
}
