use base64::{engine::general_purpose::STANDARD, Engine};
use minisign::{sign, KeyPair as KP, SecretKey, SecretKeyBox};
use std::{
    fs::OpenOptions,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    str,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::util;

#[derive(Clone, Debug)]
pub struct KeyPair {
    pub pk: String,
    pub sk: String,
}

pub fn generate_key(password: Option<String>) -> crate::Result<KeyPair> {
    let KP { pk, sk } = KP::generate_encrypted_keypair(password).unwrap();

    let pk_box_str = pk.to_box().unwrap().to_string();
    let sk_box_str = sk.to_box(None).unwrap().to_string();

    let encoded_pk = base64::engine::general_purpose::STANDARD.encode(pk_box_str);
    let encoded_sk = base64::engine::general_purpose::STANDARD.encode(sk_box_str);

    Ok(KeyPair {
        pk: encoded_pk,
        sk: encoded_sk,
    })
}

pub fn decode_base64(base64_key: &str) -> crate::Result<String> {
    let decoded_str = &base64::engine::general_purpose::STANDARD.decode(base64_key)?[..];
    Ok(String::from(str::from_utf8(decoded_str)?))
}

pub fn decode_private_key(private_key: &str, password: Option<&str>) -> crate::Result<SecretKey> {
    let decoded_secret = decode_base64(private_key)?;
    let sk_box = SecretKeyBox::from_string(&decoded_secret)?;
    let sk = sk_box.into_secret_key(password.map(Into::into))?;
    Ok(sk)
}

pub fn save_keypair<P: AsRef<Path>>(
    keypair: &KeyPair,
    path: P,
    force: bool,
) -> crate::Result<(PathBuf, PathBuf)> {
    let path = path.as_ref();

    let pubkey_path = format!("{}.pub", path.display());
    let pk_path = Path::new(&pubkey_path);

    if path.exists() {
        if !force {
            return Err(crate::Error::SigningKeyExists(path.to_path_buf()));
        } else {
            std::fs::remove_file(path)?;
        }
    }

    if pk_path.exists() {
        std::fs::remove_file(pk_path)?;
    }

    let mut sk_writer = util::create_file(path)?;
    write!(sk_writer, "{}", keypair.sk)?;
    sk_writer.flush()?;

    let mut pk_writer = util::create_file(pk_path)?;
    write!(pk_writer, "{}", keypair.pk)?;
    pk_writer.flush()?;

    Ok((dunce::canonicalize(path)?, dunce::canonicalize(pk_path)?))
}

/// Signing configuration.
#[derive(Debug, Clone)]
pub struct SigningConfig {
    /// The private key to use for signing.
    pub private_key: String,
    /// The private key password.
    ///
    /// If `None`, user will be prompted to write a password.
    /// You can skip the prompt by specifying an empty string.
    pub password: Option<String>,
}

pub fn sign_file<P: AsRef<Path>>(config: &SigningConfig, path: P) -> crate::Result<PathBuf> {
    let secret_key = decode_private_key(&config.private_key, config.password.as_deref())?;

    let path = path.as_ref();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let signature_path = path.with_extension(format!("{}.sig", extension));
    let signature_path = dunce::simplified(&signature_path);

    let mut signature_box_writer = util::create_file(signature_path)?;
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH)?.as_secs();
    let trusted_comment = format!(
        "timestamp:{}\tfile:{}",
        since_epoch,
        path.file_name().unwrap().to_string_lossy()
    );

    let file = OpenOptions::new().read(true).open(path)?;
    let file_reader = BufReader::new(file);

    let signature_box = sign(
        None,
        &secret_key,
        file_reader,
        Some(trusted_comment.as_str()),
        Some("signature from tauri secret key"),
    )?;

    let encoded_signature = STANDARD.encode(signature_box.to_string());
    signature_box_writer.write_all(encoded_signature.as_bytes())?;
    signature_box_writer.flush()?;

    Ok(dunce::canonicalize(signature_path)?)
}
