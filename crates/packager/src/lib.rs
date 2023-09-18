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
mod nsis;
mod shell;
mod sign;
pub mod util;
#[cfg(windows)]
mod wix;

use std::{path::PathBuf, process::Command};

pub use config::{Config, PackageFormat};
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
    cmd.arg("/S").arg("/C").arg(script);
    #[cfg(not(windows))]
    let mut cmd = Command::new("sh");
    cmd.current_dir(dunce::canonicalize(std::env::current_dir().unwrap()).unwrap());
    #[cfg(not(windows))]
    cmd.arg("-c").arg(script);
    cmd
}

pub fn package(config: &Config) -> Result<Vec<Package>> {
    let mut packages = Vec::new();

    let mut formats = config
        .formats
        .clone()
        .unwrap_or_else(|| PackageFormat::platform_defaults().to_vec());
    formats.sort_by_key(|f| f.priority());

    let formats_comma_separated = formats
        .iter()
        .map(|f| f.short_name())
        .collect::<Vec<_>>()
        .join(",");

    if !formats.is_empty() {
        if let Some(hook) = &config.before_packaging_command {
            let (mut cmd, script) = match hook {
                cargo_packager_config::HookCommand::Script(script) => {
                    let cmd = cross_command(script);
                    (cmd, script)
                }
                cargo_packager_config::HookCommand::ScriptWithOptions { script, dir } => {
                    let mut cmd = cross_command(script);
                    if let Some(dir) = dir {
                        cmd.current_dir(dir);
                    }
                    (cmd, script)
                }
            };

            log::info!(action = "Running"; "beforePackagingCommand `{}`", script);
            let status = cmd
                .env("CARGO_PACKAGER_FORMATS", &formats_comma_separated)
                .status()?;

            if !status.success() {
                return Err(crate::Error::HookCommandFailure(
                    "beforePackagingCommand".into(),
                    script.into(),
                    status.code().unwrap_or_default(),
                ));
            }
        }
    }

    for format in &formats {
        if let Some(hook) = &config.before_each_package_command {
            let (mut cmd, script) = match hook {
                cargo_packager_config::HookCommand::Script(script) => {
                    let cmd = cross_command(script);
                    (cmd, script)
                }
                cargo_packager_config::HookCommand::ScriptWithOptions { script, dir } => {
                    let mut cmd = cross_command(script);
                    if let Some(dir) = dir {
                        cmd.current_dir(dir);
                    }
                    (cmd, script)
                }
            };

            log::info!(action = "Running"; "[\x1b[34m{}\x1b[0m] beforeEachPackageCommand `{}`", format, script);
            let status = cmd
                .env("CARGO_PACKAGER_FORMATS", &formats_comma_separated)
                .env("CARGO_PACKAGER_FORMAT", format.short_name())
                .status()?;

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
            PackageFormat::Dmg => {
                // PackageFormat::App is required for the DMG bundle
                if !packages
                    .iter()
                    .any(|b: &Package| b.format == PackageFormat::App)
                {
                    let paths = app::package(config)?;
                    packages.push(Package {
                        format: PackageFormat::App,
                        paths,
                    });
                }
                dmg::package(config)
            }
            #[cfg(target_os = "windows")]
            PackageFormat::Wix => wix::package(config),
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
            PackageFormat::AppImage => appimage::package(config),

            _ => {
                log::warn!("ignoring {}", format.short_name());
                continue;
            }
        }?;

        packages.push(Package {
            format: *format,
            paths,
        });
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
