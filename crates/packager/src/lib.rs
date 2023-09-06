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

#[cfg(target_os = "macos")]
mod app;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod appimage;
pub mod config;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod deb;
#[cfg(target_os = "macos")]
mod dmg;
mod error;
#[cfg(target_os = "macos")]
mod ios;
#[cfg(windows)]
mod msi;
mod nsis;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod rpm;
mod sign;
pub mod util;

use std::{path::PathBuf, process::Command};

use config::Config;
pub use config::PackageFormat;
pub use error::{Error, Result};

/// Generated Package metadata.
#[derive(Debug)]
pub struct Package {
    /// The package type.
    pub format: PackageFormat,
    /// All paths for this package.
    pub paths: Vec<PathBuf>,
}

fn cross_command(script: &str) -> Command {
    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    cmd.arg("/S").arg("/C").arg(&script);
    #[cfg(not(windows))]
    let mut cmd = Command::new("sh");
    #[cfg(not(windows))]
    cmd.arg("-c").arg(&script);
    cmd
}

pub fn package(config: &Config) -> Result<Vec<Package>> {
    let target_os = config
        .target_triple
        .split('-')
        .nth(2)
        .unwrap_or(std::env::consts::OS)
        .replace("darwin", "macos");

    if target_os != std::env::consts::OS {
        log:: warn!("Cross-platform compilation is experimental and does not support all features. Please use a matching host system for full compatibility.");
    }

    let mut packages = Vec::new();

    let formats = config
        .format
        .clone()
        .unwrap_or_else(|| PackageFormat::all().to_vec());

    if !formats.is_empty() {
        if let Some(hook) = &config.before_packaging_command {
            let (mut cmd, script) = match hook {
                cargo_packager_config::HookCommand::Script(script) => {
                    let cmd = cross_command(&script);
                    (cmd, script)
                }
                cargo_packager_config::HookCommand::ScriptWithOptions { script, dir } => {
                    let mut cmd = cross_command(&script);
                    if let Some(dir) = dir {
                        cmd.current_dir(dir);
                    }
                    (cmd, script)
                }
            };

            log::info!(action = "Running"; "beforePackagingCommand `{}`", script);
            let status = cmd.status()?;

            if !status.success() {
                return Err(crate::Error::HookCommandFailure(
                    "beforePackagingCommand".into(),
                    script.into(),
                    status.code().unwrap_or_default(),
                ));
            }
        }
    }

    for format in formats {
        if let Some(hook) = &config.before_each_package_command {
            let (mut cmd, script) = match hook {
                cargo_packager_config::HookCommand::Script(script) => {
                    let cmd = cross_command(&script);
                    (cmd, script)
                }
                cargo_packager_config::HookCommand::ScriptWithOptions { script, dir } => {
                    let mut cmd = cross_command(&script);
                    if let Some(dir) = dir {
                        cmd.current_dir(dir);
                    }
                    (cmd, script)
                }
            };

            log::info!(action = "Running"; "[\x1b[34m{}\x1b[0m] beforeEachPackageCommand `{}`", format, script);
            let status = cmd.status()?;

            if !status.success() {
                return Err(crate::Error::HookCommandFailure(
                    "beforeEachPackageCommand".into(),
                    script.into(),
                    status.code().unwrap_or_default(),
                ));
            }
        }

        let paths = match format {
            #[cfg(target_os = "macos")]
            PackageFormat::App => app::package(config),
            #[cfg(target_os = "macos")]
            PackageFormat::Dmg => dmg::package(config),
            #[cfg(target_os = "macos")]
            PackageFormat::Ios => ios::package(config),
            #[cfg(target_os = "windows")]
            PackageFormat::Msi => msi::package(config),
            PackageFormat::Nsis => nsis::package(config),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Deb => deb::package(config),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Rpm => rpm::package(config),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::AppImage => appimage::package(config),

            _ => {
                log::warn!("ignoring {}", format.short_name());
                continue;
            }
        }?;

        packages.push(Package { format, paths });
    }

    #[cfg(target_os = "macos")]
    {
        // Clean up .app if only building dmg
        if !formats.contains(&PackageFormat::App) {
            if let Some(app_bundle_paths) = packages
                .iter()
                .position(|b| b.format == PackageFormat::App)
                .map(|i| packages.remove(i))
                .map(|b| b.paths)
            {
                for path in &app_bundle_paths {
                    log::debug!(action = "Cleaning"; "{}", path.display());
                    match path.is_dir() {
                        true => std::fs::remove_dir_all(path)?,
                        false => std::fs::remove_file(path)?,
                    };
                }
            }
        }
    }

    Ok(packages)
}
