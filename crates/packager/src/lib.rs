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

use std::path::{Path, PathBuf};

use config::ConfigExt;
pub use config::{Config, PackageFormat};
pub use error::{Error, Result};

/// The packaging context info
pub(crate) struct Context<'a> {
    /// The config for the app we are packaging
    pub(crate) config: &'a Config,
    /// The intermediates path, which is `<out-dir>/.cargo-packager`
    pub(crate) intermediates_path: &'a Path,
    /// The global path which we store tools used by cargo-packager and usually is
    /// `<cache-dir>/.cargo-packager`
    pub(crate) tools_path: &'a Path,
}

/// Generated Package metadata.
#[derive(Debug)]
pub struct PackageOuput {
    /// The package type.
    pub format: PackageFormat,
    /// All paths for this package.
    pub paths: Vec<PathBuf>,
}

pub fn package(config: &Config) -> Result<Vec<PackageOuput>> {
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
                    let cmd = util::cross_command(script);
                    (cmd, script)
                }
                cargo_packager_config::HookCommand::ScriptWithOptions { script, dir } => {
                    let mut cmd = util::cross_command(script);
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

    let tools_path = dirs::cache_dir()
        .unwrap_or_else(|| config.out_dir())
        .join(".cargo-packager");
    if !tools_path.exists() {
        std::fs::create_dir_all(&tools_path)?;
    }

    let intermediates_path = config.out_dir().join(".cargo-packager");
    util::create_clean_dir(&intermediates_path)?;

    let ctx = Context {
        config,
        tools_path: &tools_path,
        intermediates_path: &intermediates_path,
    };

    for format in &formats {
        if let Some(hook) = &config.before_each_package_command {
            let (mut cmd, script) = match hook {
                cargo_packager_config::HookCommand::Script(script) => {
                    let cmd = util::cross_command(script);
                    (cmd, script)
                }
                cargo_packager_config::HookCommand::ScriptWithOptions { script, dir } => {
                    let mut cmd = util::cross_command(script);
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
            PackageFormat::App => app::package(&ctx),
            #[cfg(target_os = "macos")]
            PackageFormat::Dmg => {
                // PackageFormat::App is required for the DMG bundle
                if !packages
                    .iter()
                    .any(|b: &PackageOuput| b.format == PackageFormat::App)
                {
                    let paths = app::package(&ctx)?;
                    packages.push(PackageOuput {
                        format: PackageFormat::App,
                        paths,
                    });
                }
                dmg::package(&ctx)
            }
            #[cfg(target_os = "windows")]
            PackageFormat::Wix => wix::package(&ctx),
            PackageFormat::Nsis => nsis::package(&ctx),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Deb => deb::package(&ctx),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::AppImage => appimage::package(&ctx),

            _ => {
                log::warn!("ignoring {}", format.short_name());
                continue;
            }
        }?;

        packages.push(PackageOuput {
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
