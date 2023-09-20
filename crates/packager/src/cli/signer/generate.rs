use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Generate a new signing key to sign files")]
pub struct Options {
    /// Set a password for the new signing key.
    #[clap(long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD")]
    password: Option<String>,
    /// A path where the private key will be stored.
    path: Option<PathBuf>,
    /// Overwrite the private key even if it exists on the specified path.
    #[clap(short, long)]
    force: bool,
    /// Run in CI mode and skip prompting for values.
    #[clap(long)]
    ci: bool,
}

pub fn command(mut options: Options) -> crate::Result<()> {
    options.ci = options.ci || std::env::var("CI").is_ok();
    if options.ci && options.password.is_none() {
        log::warn!("Generating a new private key without a password, for security reasons, we recommend setting a password instead.");
        options.password.replace("".into());
    }

    log::info!(action = "Generating"; "a new signgin key.");
    let keypair = crate::sign::generate_key(options.password)?;

    match options.path {
        Some(path) => {
            let keys = crate::sign::save_keypair(&keypair, path, options.force)?;
            log::info!(action = "Finished"; "generating and saving the keys:\n        {}\n        {}", keys.0.display(),keys.1.display());
        }
        None => {
            log::info!(action = "Finished"; "generating secret key:\n{}", keypair.sk);
            log::info!(action = "Finished"; "generating publick key:\n{}", keypair.pk);
        }
    }

    Ok(())
}
