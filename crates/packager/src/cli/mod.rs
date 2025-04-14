// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! The cli entry point

use std::{ffi::OsString, fmt::Write, fs, path::PathBuf};

use clap::{ArgAction, CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::{
    config::{LogLevel, PackageFormat},
    init_tracing_subscriber, package, parse_log_level, sign_outputs, util, SigningConfig,
};

mod config;
mod error;
mod signer;

use self::error::{Error, Result};

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
    #[clap(short, long, global = true)]
    quite: bool,

    /// The package fromats to build.
    #[clap(short, long, value_enum, value_delimiter = ',')]
    formats: Option<Vec<PackageFormat>>,
    /// A configuration to read, which could be a JSON file,
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
    /// Which packages to use from the current workspace.
    #[clap(short, long, value_delimiter = ',')]
    pub(crate) packages: Option<Vec<String>>,
    /// The directory where the packages will be placed.
    ///
    /// If [`Config::binaries_dir`] is not defined, it is also the path where the binaries are located if they use relative paths.
    #[clap(short, long, alias = "out")]
    out_dir: Option<PathBuf>,
    /// The directory where the [`Config::binaries`] exist.
    ///
    /// Defaults to [`Config::out_dir`]
    #[clap(long)]
    binaries_dir: Option<PathBuf>,
    /// Package the release version of your app.
    /// Ignored when `--config` is used.
    #[clap(short, long, group = "cargo-profile")]
    release: bool,
    /// Cargo profile to use for packaging your app.
    /// Ignored when `--config` is used.
    #[clap(long, group = "cargo-profile")]
    profile: Option<String>,
    /// Path to Cargo.toml manifest path to use for reading the configuration.
    /// Ignored when `--config` is used.
    #[clap(long)]
    manifest_path: Option<PathBuf>,
    /// Target triple to use for detecting your app binaries.
    #[clap(long)]
    target: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[tracing::instrument(level = "trace", skip(cli))]
fn run_cli(cli: Cli) -> Result<()> {
    tracing::trace!(cli= ?cli);

    // run subcommand and exit if one was specified,
    // otherwise run the default packaging command
    if let Some(command) = cli.command {
        match command {
            Commands::Signer(opts) => signer::command(opts)?,
        }
        return Ok(());
    }

    let configs = config::detect_configs(&cli)?;

    if configs.is_empty() {
        tracing::error!("Couldn't detect a valid configuration file or all configurations are disabled! Nothing to do here.");
        std::process::exit(1);
    }

    let cli_out_dir = cli
        .out_dir
        .as_ref()
        .map(|p| {
            if p.exists() {
                dunce::canonicalize(p).map_err(|e| Error::IoWithPath(p.clone(), e))
            } else {
                fs::create_dir_all(p).map_err(|e| Error::IoWithPath(p.clone(), e))?;
                Ok(p.to_owned())
            }
        })
        .transpose()?;

    let private_key = match cli.private_key {
        Some(path) if PathBuf::from(&path).exists() => Some(
            fs::read_to_string(&path).map_err(|e| Error::IoWithPath(PathBuf::from(&path), e))?,
        ),
        k => k,
    };

    let signing_config = private_key.map(|k| SigningConfig {
        private_key: k,
        password: cli.password,
    });

    let mut outputs = Vec::new();
    let mut signatures = Vec::new();
    for (config_dir, mut config) in configs {
        tracing::trace!(config = ?config);

        if let Some(dir) = &cli_out_dir {
            config.out_dir.clone_from(dir)
        }

        if let Some(formats) = &cli.formats {
            config.formats.replace(formats.clone());
        }

        if let Some(target_triple) = &cli.target {
            config.target_triple.replace(target_triple.clone());
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

        if let Some(path) = config_dir {
            // change the directory to the config being built
            // so paths will be read relative to it
            let parent = path
                .parent()
                .ok_or_else(|| crate::Error::ParentDirNotFound(path.clone()))?;
            std::env::set_current_dir(parent)
                .map_err(|e| Error::IoWithPath(parent.to_path_buf(), e))?;
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
        if std::env::var_os("CARGO_TERM_COLOR").is_none() {
            std::env::set_var("CARGO_TERM_COLOR", "always");
        }
    }

    run_cli(cli)
}
