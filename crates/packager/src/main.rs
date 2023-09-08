use std::{io::Write, path::PathBuf};

use cargo_packager::{
    config::{Binary, Config},
    package, util, Result,
};
use cargo_packager_config::{LogLevel, PackageFormat};
use clap::{ArgAction, CommandFactory, FromArgMatches, Parser};
use env_logger::fmt::Color;
use log::{log_enabled, Level};

fn load_configs_from_cwd(profile: &str, cli: &Cli) -> Result<Vec<(PathBuf, Config)>> {
    let mut metadata_cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest_path) = &cli.manifest_path {
        metadata_cmd.manifest_path(manifest_path);
    }
    let metadata = metadata_cmd.exec()?;

    let mut configs = Vec::new();
    for package in metadata.workspace_packages().iter() {
        // skip if this package was not specified in the explicit packages to build
        if cli
            .packages
            .as_ref()
            .map(|packages| !packages.contains(&package.name))
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(config) = package.metadata.get("packager") {
            let mut config: Config = serde_json::from_value(config.to_owned())?;
            if let Some(formats) = &cli.formats {
                config.formats.replace(formats.clone());
            }
            if config.product_name.is_empty() {
                config.product_name = package.name.clone();
            }
            if config.version.is_empty() {
                config.version = package.version.to_string();
            }
            if config.out_dir.as_os_str().is_empty() {
                config.out_dir = metadata
                    .target_directory
                    .as_std_path()
                    .to_path_buf()
                    .join(profile);
            }
            if config.description.is_none() {
                config.description = package.description.clone();
            }
            if config.authors.is_empty() {
                config.authors = package.authors.clone();
            }
            if config.default_run.is_none() {
                config.default_run = package.default_run.clone();
            }
            if config.target_triple.is_empty() {
                config.target_triple = util::target_triple()?;
            }
            if config.log_level.is_none() {
                config.log_level.replace(match cli.verbose {
                    0 => LogLevel::Info,
                    1 => LogLevel::Debug,
                    2.. => LogLevel::Trace,
                });
            }
            if config.license_file.is_none() {
                config.license_file = package
                    .license_file
                    .as_ref()
                    .map(|p| p.as_std_path().to_owned());
            }
            let targets = package
                .targets
                .iter()
                .filter(|t| t.is_bin())
                .collect::<Vec<_>>();
            for target in &targets {
                config.binaries.push(Binary {
                    name: target.name.clone(),
                    path: target.src_path.as_std_path().to_owned(),
                    main: match targets.len() {
                        1 => true,
                        _ => target.name == package.name,
                    },
                })
            }
            configs.push((package.manifest_path.as_std_path().to_path_buf(), config));
        }
    }

    Ok(configs)
}

#[derive(Parser)]
#[clap(
    author,
    version,
    about,
    bin_name("cargo-packager"),
    propagate_version(true),
    no_binary_name(true)
)]
pub(crate) struct Cli {
    /// Enables verbose logging
    #[clap(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,
    /// Package your app in release mode.
    #[clap(short, long)]
    release: bool,
    /// Specify the cargo profile to use for packaging your app.
    #[clap(long)]
    profile: Option<String>,
    /// Specify the packages to build.
    #[clap(short, long)]
    packages: Option<Vec<String>>,
    /// Specify the manifest path to use for reading the configuration.
    #[clap(long)]
    manifest_path: Option<String>,
    /// Specify the package fromats to build.
    #[clap(long, value_enum)]
    formats: Option<Vec<PackageFormat>>,
}

fn try_run(cli: Cli) -> Result<()> {
    use std::fmt::Write;

    let profile = if cli.release {
        "release"
    } else if let Some(profile) = &cli.profile {
        profile.as_str()
    } else {
        "debug"
    };

    let mut outputs = Vec::new();
    for (manifest_path, config) in load_configs_from_cwd(profile, &cli)? {
        // change the directory to the manifest being built
        // so paths are read relative to it
        std::env::set_current_dir(
            manifest_path
                .parent()
                .ok_or(cargo_packager::Error::ParentDirNotFound)?,
        )?;

        // create the packages
        outputs.extend(package(&config)?);
    }

    // print information when finished
    let len = outputs.len();
    if len >= 1 {
        let pluralised = if len == 1 { "package" } else { "packages" };
        let mut printable_paths = String::new();
        for p in outputs {
            for path in &p.paths {
                writeln!(printable_paths, "        {}", util::display_path(path)).unwrap();
            }
        }
        log::info!(action = "Finished"; "{} {} at:\n{}", len, pluralised, printable_paths);
    }

    Ok(())
}

fn main() {
    // prepare cli args
    let args = std::env::args_os().skip(1);
    let cli = Cli::command();
    let matches = cli.get_matches_from(args);
    let res = Cli::from_arg_matches(&matches).map_err(|e| e.format(&mut Cli::command()));
    let cli = match res {
        Ok(s) => s,
        Err(e) => e.exit(),
    };

    // setup logger
    let filter_level = match cli.verbose {
        0 => Level::Info,
        1 => Level::Debug,
        2.. => Level::Trace,
    }
    .to_level_filter();
    let mut builder = env_logger::Builder::from_default_env();
    let logger_init_res = builder
        .format_indent(Some(12))
        .filter(None, filter_level)
        .format(|f, record| {
            let mut is_command_output = false;
            if let Some(action) = record.key_values().get("action".into()) {
                let action = action.to_str().unwrap();
                is_command_output = action == "stdout" || action == "stderr";
                if !is_command_output {
                    let mut action_style = f.style();
                    action_style.set_color(Color::Green).set_bold(true);
                    write!(f, "{:>12} ", action_style.value(action))?;
                }
            } else {
                let mut level_style = f.default_level_style(record.level());
                level_style.set_bold(true);
                let level = match record.level() {
                    Level::Error => "Error",
                    Level::Warn => "Warn",
                    Level::Info => "Info",
                    Level::Debug => "Debug",
                    Level::Trace => "Trace",
                };
                write!(f, "{:>12} ", level_style.value(level))?;
            }

            if !is_command_output && log_enabled!(Level::Debug) {
                let mut target_style = f.style();
                target_style.set_color(Color::Black);
                write!(f, "[{}] ", target_style.value(record.target()))?;
            }

            writeln!(f, "{}", record.args())
        })
        .try_init();

    if let Err(err) = logger_init_res {
        eprintln!("Failed to attach logger: {err}");
    }

    if let Err(e) = try_run(cli) {
        log::error!("{}", e);
        std::process::exit(1);
    }
}
