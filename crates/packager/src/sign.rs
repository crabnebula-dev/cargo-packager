// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! File singing and signing keys creation and decoding.

use std::{
    fmt::Debug,
    fs::{self, OpenOptions},
    io::{BufReader, Write},
    path::{Path, PathBuf},
    str,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::{
    util::{self, PathExt},
    Error,
};

/// A public and secret key pair.
#[derive(Clone, Debug)]
pub struct KeyPair {
    /// Publick key
    pub pk: String,
    /// Secret key
    pub sk: String,
}

/// Generates a new signing key. If `password` is `None`, it will prompt
/// the user for a password, so if you want to skip the prompt, specify and
/// empty string as the password.
#[tracing::instrument(level = "trace")]
pub fn generate_key(password: Option<String>) -> crate::Result<KeyPair> {
    let minisign::KeyPair { pk, sk } = if matches!(&password, Some(p) if p.is_empty()) {
        minisign::KeyPair::generate_unencrypted_keypair()?
    } else {
        minisign::KeyPair::generate_encrypted_keypair(password)?
    };

    let pk_box_str = pk.to_box()?.to_string();
    let sk_box_str = sk.to_box(None)?.to_string();

    let encoded_pk = base64::engine::general_purpose::STANDARD.encode(pk_box_str);
    let encoded_sk = base64::engine::general_purpose::STANDARD.encode(sk_box_str);

    Ok(KeyPair {
        pk: encoded_pk,
        sk: encoded_sk,
    })
}

fn decode_base64(base64_key: &str) -> crate::Result<String> {
    let decoded_str = &base64::engine::general_purpose::STANDARD.decode(base64_key)?[..];
    Ok(String::from(str::from_utf8(decoded_str)?))
}

/// Decodes a private key using the specified password.
#[tracing::instrument(level = "trace")]
pub fn decode_private_key(
    private_key: &str,
    password: Option<&str>,
) -> crate::Result<minisign::SecretKey> {
    let decoded_secret = decode_base64(private_key)?;

    // Empty password: bypass into_secret_key() which corrupts unencrypted keys.
    if matches!(password, Some(p) if p.is_empty()) {
        let mut lines = decoded_secret.lines();
        lines.next(); // skip comment line
        if let Some(encoded_key) = lines.next() {
            if let Ok(key_bytes) = STANDARD.decode(encoded_key.trim()) {
                if let Ok(sk) = minisign::SecretKey::from_bytes(&key_bytes) {
                    if !sk.is_encrypted() {
                        return Ok(sk);
                    }
                }
            }
        }
    }

    let sk_box = minisign::SecretKeyBox::from_string(&decoded_secret)?;
    let sk = sk_box.into_secret_key(password.map(Into::into))?;
    Ok(sk)
}

/// Saves a [`KeyPair`] to disk.
#[tracing::instrument(level = "trace")]
pub fn save_keypair<P: AsRef<Path> + Debug>(
    keypair: &KeyPair,
    path: P,
    force: bool,
) -> crate::Result<(PathBuf, PathBuf)> {
    let path = path.as_ref();

    let pubkey_path = format!("{}.pub", path.display());
    let pk_path = Path::new(&pubkey_path);

    if path.exists() {
        if !force {
            return Err(Error::SigningKeyExists(path.to_path_buf()));
        } else {
            fs::remove_file(path).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?;
        }
    }

    if pk_path.exists() {
        fs::remove_file(pk_path).map_err(|e| Error::IoWithPath(pk_path.to_path_buf(), e))?;
    }

    let mut sk_writer = util::create_file(path)?;
    write!(sk_writer, "{}", keypair.sk)?;
    sk_writer.flush()?;

    let mut pk_writer = util::create_file(pk_path)?;
    write!(pk_writer, "{}", keypair.pk)?;
    pk_writer.flush()?;

    Ok((
        dunce::canonicalize(path).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?,
        dunce::canonicalize(pk_path).map_err(|e| Error::IoWithPath(pk_path.to_path_buf(), e))?,
    ))
}

/// Signing configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct SigningConfig {
    /// The private key to use for signing.
    pub private_key: String,
    /// The private key password.
    ///
    /// If `None`, user will be prompted to write a password.
    /// You can skip the prompt by specifying an empty string.
    pub password: Option<String>,
}

impl SigningConfig {
    /// Creates a new [`SigningConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the private key to use for signing.
    pub fn private_key<S: Into<String>>(mut self, private_key: S) -> Self {
        self.private_key = private_key.into();
        self
    }

    /// Set the private key password.
    pub fn password<S: Into<String>>(mut self, password: S) -> Self {
        self.password.replace(password.into());

        self
    }
}

/// Signs a specified file using the specified signing configuration.
#[tracing::instrument(level = "trace")]
pub fn sign_file<P: AsRef<Path> + Debug>(
    config: &SigningConfig,
    path: P,
) -> crate::Result<PathBuf> {
    let secret_key = decode_private_key(&config.private_key, config.password.as_deref())?;
    sign_file_with_secret_key(&secret_key, path)
}

/// Signs a specified file using an already decoded secret key.
#[tracing::instrument(level = "trace")]
pub fn sign_file_with_secret_key<P: AsRef<Path> + Debug>(
    secret_key: &minisign::SecretKey,
    path: P,
) -> crate::Result<PathBuf> {
    let path = path.as_ref();
    let signature_path = path.with_additional_extension("sig");
    let signature_path = dunce::simplified(&signature_path);

    let mut signature_box_writer = util::create_file(signature_path)?;
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH)?.as_secs();
    let trusted_comment = format!(
        "timestamp:{}\tfile:{}",
        since_epoch,
        path.file_name()
            .ok_or_else(|| crate::Error::FailedToExtractFilename(path.to_path_buf()))?
            .to_string_lossy()
    );

    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?;
    let file_reader = BufReader::new(file);

    let signature_box = minisign::sign(
        None,
        secret_key,
        file_reader,
        Some(trusted_comment.as_str()),
        Some("signature from cargo-packager secret key"),
    )?;

    let encoded_signature = STANDARD.encode(signature_box.to_string());
    signature_box_writer.write_all(encoded_signature.as_bytes())?;
    signature_box_writer.flush()?;

    dunce::canonicalize(signature_path).map_err(|e| crate::Error::IoWithPath(path.to_path_buf(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Key generated with minisign ≤0.7 + empty password (kdf_alg = "Sc", plaintext data)
    const LEGACY_SK: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5VU1qSHBMT0E4R0JCVGZzbUMzb3ZXeGpGY1NSdm9OaUxaVTFuajd0T2ZKZ0FBQkFBQUFBQUFBQUFBQUlBQUFBQWlhRnNPUmxKWjBiWnJ6M29Cd0RwOUpqTW1yOFFQK3JTOGdKSi9CajlHZktHajI2ZnprbEM0VUl2MHhGdFdkZWpHc1BpTlJWK2hOTWo0UVZDemMvaFlYVUM4U2twRW9WV1JHenNzUkRKT2RXQ1FCeXlkYUwxelhacmtxOGZJOG1Nb1R6b0VEcWFLVUk9Cg==";
    const LEGACY_PK: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ2Njc0OTE5Mzk2Q0ExODkKUldTSm9XdzVHVWxuUmtJdjB4RnRXZGVqR3NQaU5SVitoTk1qNFFWQ3pjL2hZWFVDOFNrcEVvVlcK";

    // Key generated with minisign 0.9 generate_unencrypted_keypair() (kdf_alg = KDF_NONE)
    const CURRENT_SK: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUUFBRUl5QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQTRhZUlLZEhNS2lqMmRiNWVDNDdBa1FGTFpEVG5DTGJWNjBUaGVzQTFvTHkvcjJ1U01Oa2JMSjZNRHZYdU83SHEydjJXZnRheEhvNmRDOHhYWVFlZ1lRTDBteWxQanJwNENmTUxZQVo0K2FVZE1Ia2Vtbzlld0c5ZVVzcklGQjhpUlNoVmtJbTFRZFk9Cg==";
    const CURRENT_PK: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDI4MkFDQ0QxMjk4OEE3RTEKUldUaHA0Z3AwY3dxS0o2TUR2WHVPN0hxMnYyV2Z0YXhIbzZkQzh4WFlRZWdZUUwwbXlsUGpycDQK";

    #[test]
    fn sign_verify_legacy_key() {
        let sk = decode_private_key(LEGACY_SK, Some("")).unwrap();
        assert!(!sk.is_encrypted());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, b"test content").unwrap();
        let sig_path = sign_file_with_secret_key(&sk, &path).unwrap();

        let pk_str = str::from_utf8(&STANDARD.decode(LEGACY_PK).unwrap())
            .unwrap()
            .to_owned();
        let pk = minisign::PublicKeyBox::from_string(&pk_str)
            .unwrap()
            .into_public_key()
            .unwrap();
        let sig_str = str::from_utf8(
            &STANDARD
                .decode(fs::read_to_string(&sig_path).unwrap())
                .unwrap(),
        )
        .unwrap()
        .to_owned();
        let sig_box = minisign::SignatureBox::from_string(&sig_str).unwrap();
        minisign::verify(
            &pk,
            &sig_box,
            fs::File::open(&path).unwrap(),
            true,
            false,
            false,
        )
        .unwrap();
    }

    #[test]
    fn sign_verify_current_key() {
        let sk = decode_private_key(CURRENT_SK, Some("")).unwrap();
        assert!(!sk.is_encrypted());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, b"test content").unwrap();
        let sig_path = sign_file_with_secret_key(&sk, &path).unwrap();

        let pk_str = str::from_utf8(&STANDARD.decode(CURRENT_PK).unwrap())
            .unwrap()
            .to_owned();
        let pk = minisign::PublicKeyBox::from_string(&pk_str)
            .unwrap()
            .into_public_key()
            .unwrap();
        let sig_str = str::from_utf8(
            &STANDARD
                .decode(fs::read_to_string(&sig_path).unwrap())
                .unwrap(),
        )
        .unwrap()
        .to_owned();
        let sig_box = minisign::SignatureBox::from_string(&sig_str).unwrap();
        minisign::verify(
            &pk,
            &sig_box,
            fs::File::open(&path).unwrap(),
            true,
            false,
            false,
        )
        .unwrap();
    }
}
