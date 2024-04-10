// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! The cli entry point

#![cfg(feature = "cli")]

use std::{ffi::OsString, fmt::Write, path::PathBuf};

use clap::{ArgAction, CommandFactory, FromArgMatches, Parser, Subcommand};

use self::config::{find_config_files, load_configs_from_cargo_workspace, parse_config_file};
use crate::{
    config::{Config, LogLevel, PackageFormat},
    init_tracing_subscriber, package, parse_log_level, sign_outputs, util, Result, SigningConfig,
};

mod config;
mod signer;

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    Signer(signer::Options),
}

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about,
    bin_name("cargo-packager"),
    propagate_version(true),
    no_binary_name(true)
)]
pub(crate) struct Cli {
    /// Enables verbose logging.
    #[clap(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,
    /// Disables logging
    #[clap(short, long)]
    quite: bool,

    /// Specify the package fromats to build.
    #[clap(short, long, value_enum, value_delimiter = ',')]
    formats: Option<Vec<PackageFormat>>,
    /// Specify a configuration to read, which could be a JSON file,
    /// TOML file, or a raw JSON string.
    ///
    /// By default, cargo-packager looks for `{p,P}ackager.{toml,json}` and
    /// `[package.metadata.packager]` in `Cargo.toml` files.
    #[clap(short, long)]
    config: Option<String>,
    /// Load a private key from a file or a string to sign the generated ouptuts.
    #[clap(short = 'k', long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY")]
    private_key: Option<String>,
    /// The password for the signing private key.
    #[clap(long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD")]
    password: Option<String>,
    /// Specify which packages to use from the current workspace.
    #[clap(short, long, value_delimiter = ',')]
    packages: Option<Vec<String>>,
    /// Specify The directory where the packages will be placed.
    ///
    /// If [`Config::binaries_dir`] is not defined, it is also the path where the binaries are located if they use relative paths.
    out_dir: Option<PathBuf>,

    /// Package the release version of your app.
    /// Ignored when `--config` is used.
    #[clap(short, long, group = "cargo-profile")]
    release: bool,
    /// Specify the cargo profile to use for packaging your app.
    /// Ignored when `--config` is used.
    #[clap(long, group = "cargo-profile")]
    profile: Option<String>,
    /// Specify the manifest path to use for reading the configuration.
    /// Ignored when `--config` is used.
    #[clap(long)]
    manifest_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[tracing::instrument(level = "trace")]
fn run_cli(cli: Cli) -> Result<()> {
    // run subcommand and exit if one was specified,
    // otherwise run the default packaging command
    if let Some(command) = cli.command {
        match command {
            Commands::Signer(opts) => signer::command(opts)?,
        }
        return Ok(());
    }

    let mut configs = match cli.config {
        // if a raw json object
        Some(c) if c.starts_with('{') => vec![(None, serde_json::from_str::<Config>(&c)?)],
        // if a raw json array
        Some(c) if c.starts_with('[') => serde_json::from_str::<Vec<Config>>(&c)?
            .into_iter()
            .map(|c| (None, c))
            .collect(),
        // if a path to config file
        Some(c) => parse_config_file(c)?,
        // fallback to config files and cargo workspaces configs
        _ => {
            let config_files = find_config_files()?
                .into_iter()
                .filter_map(|c| parse_config_file(c).ok())
                .collect::<Vec<_>>()
                .concat();
            let cargo_configs =
                load_configs_from_cargo_workspace(cli.release, cli.profile, cli.manifest_path)?;
            [config_files, cargo_configs]
                .concat()
                .into_iter()
                .filter(|(_, c)| {
                    // skip if this config is not enabled
                    //    or if this package was in the explicit
                    //    packages list specified on the CLI,
                    // otherwise build all if no packages were specified on the CLI
                    c.enabled
                        && (cli
                            .packages
                            .as_ref()
                            .map(|cli_packages| {
                                c.name
                                    .as_ref()
                                    .map(|name| cli_packages.contains(name))
                                    .unwrap_or(false)
                            })
                            .unwrap_or(true))
                })
                .collect()
        }
    };

    if configs.is_empty() {
        tracing::debug!("Couldn't detect a valid configuration file or all configurations are disabled! Nothing to do here.");
        return Ok(());
    }

    let cli_out_dir = cli
        .out_dir
        .as_ref()
        .map(|p| {
            if p.exists() {
                dunce::canonicalize(p)
            } else {
                std::fs::create_dir_all(p)?;
                Ok(p.to_owned())
            }
        })
        .transpose()?;

    for (_, config) in &mut configs {
        if let Some(dir) = &cli_out_dir {
            config.out_dir = dir.clone()
        }

        if let Some(formats) = &cli.formats {
            config.formats.replace(formats.clone());
        }

        if config.log_level.is_none() && !cli.quite {
            let level = match parse_log_level(cli.verbose) {
                tracing::Level::ERROR => LogLevel::Error,
                tracing::Level::WARN => LogLevel::Warn,
                tracing::Level::INFO => LogLevel::Info,
                tracing::Level::DEBUG => LogLevel::Debug,
                tracing::Level::TRACE => LogLevel::Trace,
            };
            config.log_level.replace(level);
        }
    }

    let private_key = match cli.private_key {
        Some(path) if PathBuf::from(&path).exists() => std::fs::read_to_string(path).ok(),
        k => k,
    };
    let signing_config = private_key.map(|k| SigningConfig {
        private_key: k,
        password: cli.password,
    });

    let mut outputs = Vec::new();
    let mut signatures = Vec::new();
    for (config_dir, config) in configs {
        if let Some(path) = config_dir {
            // change the directory to the config being built
            // so paths will be read relative to it
            std::env::set_current_dir(
                path.parent()
                    .ok_or_else(|| crate::Error::ParentDirNotFound(path.clone()))?,
            )?;
        }

        // create the packages
        let mut packages = package(&config)?;

        // sign the packages
        if let Some(signing_config) = &signing_config {
            let s = sign_outputs(signing_config, &mut packages)?;
            signatures.extend(s);
        }

        outputs.extend(packages);
    }

    // flatten paths
    let outputs = outputs
        .into_iter()
        .flat_map(|o| o.paths)
        .collect::<Vec<_>>();

    // print information when finished
    let len = outputs.len();
    if len >= 1 {
        let pluralised = if len == 1 { "package" } else { "packages" };
        let mut printable_paths = String::new();
        for path in outputs {
            let _ = writeln!(printable_paths, "        {}", util::display_path(path));
        }
        tracing::info!(
            "Finished packaging {} {} at:\n{}",
            len,
            pluralised,
            printable_paths
        );
    }

    let len = signatures.len();
    if len >= 1 {
        let pluralised = if len == 1 { "signature" } else { "signatures" };
        let mut printable_paths = String::new();
        for path in signatures {
            let _ = writeln!(printable_paths, "        {}", util::display_path(path));
        }
        tracing::info!(
            "Finished signing packages, {} {} at:\n{}",
            len,
            pluralised,
            printable_paths
        );
    }

    Ok(())
}

/// Run the packager CLI
pub fn run<I, A>(args: I, bin_name: Option<String>)
where
    I: IntoIterator<Item = A>,
    A: Into<OsString> + Clone,
{
    if let Err(e) = try_run(args, bin_name) {
        tracing::error!("{}", e);
        std::process::exit(1);
    }
}

/// Try run the packager CLI
pub fn try_run<I, A>(args: I, bin_name: Option<String>) -> Result<()>
where
    I: IntoIterator<Item = A>,
    A: Into<OsString> + Clone,
{
    let cli = match &bin_name {
        Some(bin_name) => Cli::command().bin_name(bin_name),
        None => Cli::command(),
    };
    let matches = cli.get_matches_from(args);
    let cli = Cli::from_arg_matches(&matches).map_err(|e| {
        e.format(&mut match &bin_name {
            Some(bin_name) => Cli::command().bin_name(bin_name),
            None => Cli::command(),
        })
    })?;

    if !cli.quite {
        init_tracing_subscriber(cli.verbose);
    }

    run_cli(cli)
}
