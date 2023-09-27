use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sign a file.")]
pub struct Options {
    /// Load the private key from a file or a string.
    #[clap(short = 'k', long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY")]
    private_key: Option<String>,
    /// The password for the private key.
    #[clap(long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD")]
    password: Option<String>,
    /// The file to be signed.
    file: PathBuf,
}

pub fn command(options: Options) -> crate::Result<()> {
    let private_key = match options.private_key {
        Some(path) if PathBuf::from(&path).exists() => std::fs::read_to_string(path)?,
        Some(key) => key,
        None => {
            tracing::error!("--private-key was not specified, aborting signign.");
            std::process::exit(1);
        }
    };

    let config = crate::sign::SigningConfig {
        private_key,
        password: Some(options.password.unwrap_or_default()),
    };
    let signature_path = crate::sign::sign_file(&config, options.file)?;

    tracing::info!(
        "Signed the file successfully! find the signature at: {}",
        signature_path.display()
    );

    Ok(())
}
