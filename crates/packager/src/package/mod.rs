// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use serde::Serialize;
use url::Url;

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
    /// Package summary for `latest.json`
    pub summary: Option<PackageOutputSummary>,
}

impl PackageOutput {
    /// Creates a new package output.
    ///
    /// This is only useful if you need to sign the packages in a different process,
    /// after packaging the app and storing its paths.
    pub fn new(format: PackageFormat, paths: Vec<PathBuf>) -> Self {
        Self {
            format,
            paths,
            summary: None,
        }
    }
}

/// Summary information for this package to be included in `latest.json`
#[derive(Debug, Clone, Serialize)]
pub struct PackageOutputSummary {
    /// Download URL for the platform
    pub url: Url,
    /// Signature for the platform. If it is None then something has gone wrong
    pub signature: Option<String>,
    /// Update format
    pub format: PackageFormat,
    /// Target triple for this package
    #[serde(skip)]
    pub platform: String,
}

/// Package an app using the specified config.
#[tracing::instrument(level = "trace", skip(config))]
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
    tracing::trace!(ctx = ?ctx);

    let mut packages = Vec::new();
    for &format in &formats {
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
                        summary: None,
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

        let summary = build_package_summary(&paths, format, config)?;

        packages.push(PackageOutput {
            format,
            summary,
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
                for p in &app_bundle_paths {
                    use crate::Error;
                    use std::fs;

                    tracing::debug!("Cleaning {}", p.display());
                    match p.is_dir() {
                        true => {
                            fs::remove_dir_all(p).map_err(|e| Error::IoWithPath(p.clone(), e))?
                        }
                        false => fs::remove_file(p).map_err(|e| Error::IoWithPath(p.clone(), e))?,
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
            .output_ok_info()
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
            .output_ok_info()
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

fn build_package_summary(
    paths: &Vec<PathBuf>,
    format: PackageFormat,
    config: &Config,
) -> crate::Result<Option<PackageOutputSummary>> {
    Ok(if let Some(url) = &config.endpoint {
        let paths = paths
            .iter()
            .cloned()
            .filter_map(|path| path.file_name().and_then(|f| f.to_str().map(Into::into)))
            .collect::<Vec<String>>();

        if paths.len() == 1 {
            let artefact = paths.first().unwrap();

            let url: Url = url
                .to_string()
                // url::Url automatically url-encodes the path components
                .replace("%7B%7Bversion%7D%7D", &config.version)
                .replace("%7B%7Bartefact%7D%7D", &artefact)
                // but not query parameters
                .replace("{{version}}", &config.version)
                .replace("{{artefact}}", &artefact)
                .parse()?;

            let target_triple = config.target_triple();
            // See the updater crate for which particular target strings are required.
            let target_arch = if target_triple.starts_with("x86_64") {
                Some("x86_64")
            } else if target_triple.starts_with('i') {
                Some("i686")
            } else if target_triple.starts_with("arm") {
                Some("armv7")
            } else if target_triple.starts_with("aarch64") {
                Some("aarch64")
            } else {
                None
            };
            let target_os = config.target_os();
            match (target_arch, target_os) {
                (Some(target_arch), Some(target_os)) => {
                    let platform = format!("{target_os}-{target_arch}");

                    Some(PackageOutputSummary {
                        url,
                        format,
                        platform,
                        // Signature will be set later
                        signature: None,
                    })
                }
                _ => {
                    tracing::warn!(target_triple =?config.target_triple(), ?target_arch, ?target_os, "A package could not be summarized in latest.json because the platform string could not be determined from {target_triple}.");
                    None
                }
            }
        } else {
            // TODO: Implement logic to decide which path to publish in PackageOutputSummary when there are multiple to choose from
            tracing::warn!("A package could not be summarized in latest.json because the package format {format:?} is not yet supported.");
            None
        }
    } else {
        // No endpoint has been configured, so no summary is outputted
        None
    })
}
