// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use crate::{config, shell::CommandExt, util, Config, PackageFormat};

use self::context::Context;

mod app;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod appimage;
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
mod nsis;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod pacman;
#[cfg(windows)]
mod wix;

mod context;

/// Generated Package metadata.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PackageOutput {
    /// The package type.
    pub format: PackageFormat,
    /// All paths for this package.
    pub paths: Vec<PathBuf>,
}

impl PackageOutput {
    /// Creates a new package output.
    ///
    /// This is only useful if you need to sign the packages in a different process,
    /// after packaging the app and storing its paths.
    pub fn new(format: PackageFormat, paths: Vec<PathBuf>) -> Self {
        Self { format, paths }
    }
}

/// Package an app using the specified config.
#[tracing::instrument(level = "trace")]
pub fn package(config: &Config) -> crate::Result<Vec<PackageOutput>> {
    let mut formats = config
        .formats
        .clone()
        .unwrap_or_else(|| PackageFormat::platform_default().to_vec());

    if formats.is_empty() {
        return Ok(Vec::new());
    }

    if formats.contains(&PackageFormat::Default) {
        formats = PackageFormat::platform_default().to_vec();
    }

    if formats.contains(&PackageFormat::All) {
        formats = PackageFormat::platform_all().to_vec();
    }

    formats.sort_by_key(|f| f.priority());

    let formats_comma_separated = formats
        .iter()
        .map(|f| f.short_name())
        .collect::<Vec<_>>()
        .join(",");

    run_before_packaging_command_hook(config, &formats_comma_separated)?;

    let ctx = Context::new(config)?;

    let mut packages = Vec::new();
    for format in &formats {
        run_before_each_packaging_command_hook(
            config,
            &formats_comma_separated,
            format.short_name(),
        )?;

        let paths = match format {
            PackageFormat::App => app::package(&ctx),
            #[cfg(target_os = "macos")]
            PackageFormat::Dmg => {
                // PackageFormat::App is required for the DMG bundle
                if !packages
                    .iter()
                    .any(|b: &PackageOutput| b.format == PackageFormat::App)
                {
                    let paths = app::package(&ctx)?;
                    packages.push(PackageOutput {
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
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Pacman => pacman::package(&ctx),

            _ => {
                tracing::warn!("ignoring {}", format.short_name());
                continue;
            }
        }?;

        packages.push(PackageOutput {
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
                    tracing::debug!("Cleaning {}", path.display());
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

fn run_before_each_packaging_command_hook(
    config: &Config,
    formats_comma_separated: &str,
    format: &str,
) -> crate::Result<()> {
    if let Some(hook) = &config.before_each_package_command {
        let (mut cmd, script) = match hook {
            config::HookCommand::Script(script) => {
                let cmd = util::cross_command(script);
                (cmd, script)
            }
            config::HookCommand::ScriptWithOptions { script, dir } => {
                let mut cmd = util::cross_command(script);
                if let Some(dir) = dir {
                    cmd.current_dir(dir);
                }
                (cmd, script)
            }
        };

        tracing::info!("Running beforeEachPackageCommand [{format}] `{script}`");
        let output = cmd
            .env("CARGO_PACKAGER_FORMATS", formats_comma_separated)
            .env("CARGO_PACKAGER_FORMAT", format)
            .output_ok()
            .map_err(|e| {
                crate::Error::HookCommandFailure(
                    "beforeEachPackageCommand".into(),
                    script.into(),
                    e,
                )
            })?;

        if !output.status.success() {
            return Err(crate::Error::HookCommandFailureWithExitCode(
                "beforeEachPackageCommand".into(),
                script.into(),
                output.status.code().unwrap_or_default(),
            ));
        }
    }

    Ok(())
}

fn run_before_packaging_command_hook(
    config: &Config,
    formats_comma_separated: &str,
) -> crate::Result<()> {
    if let Some(hook) = &config.before_packaging_command {
        let (mut cmd, script) = match hook {
            config::HookCommand::Script(script) => {
                let cmd = util::cross_command(script);
                (cmd, script)
            }
            config::HookCommand::ScriptWithOptions { script, dir } => {
                let mut cmd = util::cross_command(script);
                if let Some(dir) = dir {
                    cmd.current_dir(dir);
                }
                (cmd, script)
            }
        };

        tracing::info!("Running beforePackageCommand `{script}`");
        let output = cmd
            .env("CARGO_PACKAGER_FORMATS", formats_comma_separated)
            .output_ok()
            .map_err(|e| {
                crate::Error::HookCommandFailure("beforePackagingCommand".into(), script.into(), e)
            })?;

        if !output.status.success() {
            return Err(crate::Error::HookCommandFailureWithExitCode(
                "beforePackagingCommand".into(),
                script.into(),
                output.status.code().unwrap_or_default(),
            ));
        }
    }

    Ok(())
}
