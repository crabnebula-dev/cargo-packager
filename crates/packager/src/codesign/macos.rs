// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    cmp::Ordering,
    ffi::OsString,
    fs::File,
    io::prelude::*,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;

use crate::{shell::CommandExt, Config, Error};

const KEYCHAIN_ID: &str = "cargo-packager.keychain";
const KEYCHAIN_PWD: &str = "cargo-packager";

// Import certificate from ENV variables.
// APPLE_CERTIFICATE is the p12 certificate base64 encoded.
// By example you can use; openssl base64 -in MyCertificate.p12 -out MyCertificate-base64.txt
// Then use the value of the base64 in APPLE_CERTIFICATE env variable.
// You need to set APPLE_CERTIFICATE_PASSWORD to the password you set when you exported your certificate.
// https://help.apple.com/xcode/mac/current/#/dev154b28f09 see: `Export a signing certificate`
#[tracing::instrument(level = "trace")]
pub fn setup_keychain(
    certificate_encoded: OsString,
    certificate_password: OsString,
) -> crate::Result<()> {
    // we delete any previous version of our keychain if present
    delete_keychain();

    tracing::info!("Setting up keychain from environment variables...");

    let keychain_list_output = Command::new("security")
        .args(["list-keychain", "-d", "user"])
        .output()
        .map_err(crate::Error::FailedToListKeyChain)?;

    let tmp_dir = tempfile::tempdir()?;

    let cert_path = tmp_dir
        .path()
        .join("cert.p12")
        .to_string_lossy()
        .to_string();
    let cert_path_tmp = tmp_dir
        .path()
        .join("cert.p12.tmp")
        .to_string_lossy()
        .to_string();
    let certificate_encoded = certificate_encoded
        .to_str()
        .expect("failed to convert APPLE_CERTIFICATE to string")
        .as_bytes();
    let certificate_password = certificate_password
        .to_str()
        .expect("failed to convert APPLE_CERTIFICATE_PASSWORD to string")
        .to_string();

    // as certificate contain whitespace decoding may be broken
    // https://github.com/marshallpierce/rust-base64/issues/105
    // we'll use builtin base64 command from the OS
    let mut tmp_cert = File::create(cert_path_tmp.clone())?;
    tmp_cert.write_all(certificate_encoded)?;

    Command::new("base64")
        .args(["--decode", "-i", &cert_path_tmp, "-o", &cert_path])
        .output_ok()
        .map_err(crate::Error::FailedToDecodeCert)?;

    Command::new("security")
        .args(["create-keychain", "-p", KEYCHAIN_PWD, KEYCHAIN_ID])
        .output_ok()
        .map_err(crate::Error::FailedToCreateKeyChain)?;

    Command::new("security")
        .args(["unlock-keychain", "-p", KEYCHAIN_PWD, KEYCHAIN_ID])
        .output_ok()
        .map_err(crate::Error::FailedToUnlockKeyChain)?;

    Command::new("security")
        .args([
            "import",
            &cert_path,
            "-k",
            KEYCHAIN_ID,
            "-P",
            &certificate_password,
            "-T",
            "/usr/bin/codesign",
            "-T",
            "/usr/bin/pkgbuild",
            "-T",
            "/usr/bin/productbuild",
        ])
        .output_ok()
        .map_err(crate::Error::FailedToImportCert)?;

    Command::new("security")
        .args(["set-keychain-settings", "-t", "3600", "-u", KEYCHAIN_ID])
        .output_ok()
        .map_err(crate::Error::FailedToSetKeychainSettings)?;

    Command::new("security")
        .args([
            "set-key-partition-list",
            "-S",
            "apple-tool:,apple:,codesign:",
            "-s",
            "-k",
            KEYCHAIN_PWD,
            KEYCHAIN_ID,
        ])
        .output_ok()
        .map_err(crate::Error::FailedToSetKeyPartitionList)?;

    let current_keychains = String::from_utf8_lossy(&keychain_list_output.stdout)
        .split('\n')
        .map(|line| {
            line.trim_matches(|c: char| c.is_whitespace() || c == '"')
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<String>>();

    Command::new("security")
        .args(["list-keychain", "-d", "user", "-s"])
        .args(current_keychains)
        .arg(KEYCHAIN_ID)
        .output_ok()
        .map_err(crate::Error::FailedToListKeyChain)?;

    Ok(())
}

#[tracing::instrument(level = "trace")]
pub fn delete_keychain() {
    // delete keychain if needed and skip any error
    let _ = Command::new("security")
        .arg("delete-keychain")
        .arg(KEYCHAIN_ID)
        .output_ok();
}

#[derive(Debug, PartialEq, Eq)]
pub struct SignTarget {
    pub path: PathBuf,
    pub is_native_binary: bool,
}

impl Ord for SignTarget {
    fn cmp(&self, other: &Self) -> Ordering {
        // This ordering implementation ensures that signing targets with
        // a shorter path are greater than signing targets with a longer path.
        //
        // When sorting in ascending order, this means we first get the long paths
        // and then the shorter paths (aka depth-first). This is required in order
        // for signing to work properly on macOS since more deeply nested files
        // need to be signed first.

        let mut self_iter = self.path.iter();
        let mut other_iter = other.path.iter();

        let mut self_prev = None;
        let mut other_prev = None;

        loop {
            match (self_iter.next(), other_iter.next()) {
                (Some(s), Some(o)) => {
                    self_prev = Some(s);
                    other_prev = Some(o);
                }
                // This path has less components than the other path
                // and thus should come later (Ordering greater)
                (None, Some(_)) => return Ordering::Greater,

                // This path has more components than the previous path
                // and thus should come earlier
                (Some(_), None) => return Ordering::Less,

                (None, None) => {
                    return match (self_prev, other_prev) {
                        // Compare by name
                        (Some(s), Some(o)) => s.cmp(o),

                        // See above for ordering when component size is not the same
                        (None, Some(_)) => Ordering::Greater,
                        (Some(_), None) => Ordering::Less,
                        (None, None) => Ordering::Equal,
                    };
                }
            }
        }
    }
}

impl PartialOrd for SignTarget {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[tracing::instrument(level = "trace")]
pub fn try_sign(targets: Vec<SignTarget>, identity: &str, config: &Config) -> crate::Result<()> {
    let packager_keychain = if let (Some(certificate_encoded), Some(certificate_password)) = (
        std::env::var_os("APPLE_CERTIFICATE"),
        std::env::var_os("APPLE_CERTIFICATE_PASSWORD"),
    ) {
        // setup keychain allow you to import your certificate
        // for CI build
        setup_keychain(certificate_encoded, certificate_password)?;
        true
    } else {
        false
    };

    for target in targets {
        sign(
            &target.path,
            identity,
            config,
            target.is_native_binary,
            packager_keychain,
        )?;
    }

    if packager_keychain {
        // delete the keychain again after signing
        delete_keychain();
    }

    Ok(())
}

#[tracing::instrument(level = "trace")]
fn sign(
    path_to_sign: &Path,
    identity: &str,
    config: &Config,
    is_native_binary: bool,
    packager_keychain: bool,
) -> crate::Result<()> {
    tracing::info!(
        "Codesigning {} with identity \"{}\"",
        path_to_sign.display(),
        identity
    );

    let mut args = vec!["--force", "-s", identity];

    if packager_keychain {
        args.push("--keychain");
        args.push(KEYCHAIN_ID);
    }

    if let Some(entitlements_path) = config.macos().and_then(|macos| macos.entitlements.as_ref()) {
        args.push("--entitlements");
        args.push(entitlements_path);
    }

    if is_native_binary {
        args.push("--options");
        args.push("runtime");
    }

    args.push("--timestamp");

    Command::new("codesign")
        .args(args)
        .arg(path_to_sign)
        .output_ok()
        .map_err(crate::Error::FailedToRunCodesign)?;

    Ok(())
}

#[derive(Deserialize, Debug)]
struct NotarytoolSubmitOutput {
    id: String,
    status: String,
    message: String,
}

#[tracing::instrument(level = "trace")]
pub fn notarize(
    app_bundle_path: PathBuf,
    auth: NotarizeAuth,
    config: &Config,
) -> crate::Result<()> {
    let bundle_stem = app_bundle_path
        .file_stem()
        .ok_or_else(|| crate::Error::FailedToExtractFilename(app_bundle_path.clone()))?;

    let tmp_dir = tempfile::tempdir()?;
    let zip_path = tmp_dir
        .path()
        .join(format!("{}.zip", bundle_stem.to_string_lossy()));

    let app_bundle_path_str = app_bundle_path.to_string_lossy().to_string();
    let zip_path_str = zip_path.to_string_lossy().to_string();
    let zip_args = vec![
        "-c",
        "-k",
        "--keepParent",
        "--sequesterRsrc",
        &app_bundle_path_str,
        &zip_path_str,
    ];

    // use ditto to create a PKZip almost identical to Finder
    // this remove almost 99% of false alarm in notarization
    Command::new("ditto")
        .args(zip_args)
        .output_ok()
        .map_err(crate::Error::FailedToRunDitto)?;

    // sign the zip file
    if let Some(identity) = &config
        .macos()
        .and_then(|macos| macos.signing_identity.as_ref())
    {
        try_sign(
            vec![SignTarget {
                path: zip_path.clone(),
                is_native_binary: false,
            }],
            identity,
            config,
        )?;
    };

    let zip_path_str = zip_path.to_string_lossy().to_string();

    let notarize_args = vec![
        "notarytool",
        "submit",
        &zip_path_str,
        "--wait",
        "--output-format",
        "json",
    ];

    tracing::info!("Notarizing {}", app_bundle_path.display());

    let output = Command::new("xcrun")
        .args(notarize_args)
        .notarytool_args(&auth)
        .output_ok()
        .map_err(crate::Error::FailedToRunXcrun)?;

    if !output.status.success() {
        return Err(Error::FailedToNotarize);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    if let Ok(submit_output) = serde_json::from_str::<NotarytoolSubmitOutput>(&output_str) {
        let log_message = format!(
            "Finished with status {} for id {} ({})",
            submit_output.status, submit_output.id, submit_output.message
        );
        if submit_output.status == "Accepted" {
            tracing::info!("Notarizing {}", log_message);
            staple_app(app_bundle_path)?;
            Ok(())
        } else {
            Err(Error::NotarizeRejected(log_message))
        }
    } else {
        Err(Error::FailedToParseNotarytoolOutput(
            output_str.into_owned(),
        ))
    }
}

fn staple_app(app_bundle_path: PathBuf) -> crate::Result<()> {
    let filename = app_bundle_path
        .file_name()
        .ok_or_else(|| crate::Error::FailedToExtractFilename(app_bundle_path.clone()))?
        .to_string_lossy()
        .to_string();

    let app_bundle_path_dir = app_bundle_path
        .parent()
        .ok_or_else(|| crate::Error::ParentDirNotFound(app_bundle_path.clone()))?;

    Command::new("xcrun")
        .args(vec!["stapler", "staple", "-v", &filename])
        .current_dir(app_bundle_path_dir)
        .output_ok()
        .map_err(crate::Error::FailedToRunXcrun)?;

    Ok(())
}

#[derive(Debug)]
pub enum NotarizeAuth {
    AppleId {
        apple_id: OsString,
        password: OsString,
        team_id: OsString,
    },
    ApiKey {
        key: OsString,
        key_path: PathBuf,
        issuer: OsString,
    },
}

pub trait NotarytoolCmdExt {
    fn notarytool_args(&mut self, auth: &NotarizeAuth) -> &mut Self;
}

impl NotarytoolCmdExt for Command {
    fn notarytool_args(&mut self, auth: &NotarizeAuth) -> &mut Self {
        match auth {
            NotarizeAuth::AppleId {
                apple_id,
                password,
                team_id,
            } => {
                self.arg("--apple-id")
                    .arg(apple_id)
                    .arg("--password")
                    .arg(password)
                    .arg("--team-id")
                    .arg(team_id);

                self
            }
            NotarizeAuth::ApiKey {
                key,
                key_path,
                issuer,
            } => self
                .arg("--key-id")
                .arg(key)
                .arg("--key")
                .arg(key_path)
                .arg("--issuer")
                .arg(issuer),
        }
    }
}

#[tracing::instrument(level = "trace")]
pub fn notarize_auth() -> crate::Result<NotarizeAuth> {
    match (
        std::env::var_os("APPLE_ID"),
        std::env::var_os("APPLE_PASSWORD"),
        std::env::var_os("APPLE_TEAM_ID"),
    ) {
        (Some(apple_id), Some(password), Some(team_id)) => Ok(NotarizeAuth::AppleId {
            apple_id,
            password,
            team_id,
        }),
        _ => {
            match (
                std::env::var_os("APPLE_API_KEY"),
                std::env::var_os("APPLE_API_ISSUER"),
                std::env::var("APPLE_API_KEY_PATH"),
            ) {
                (Some(key), Some(issuer), Ok(key_path)) => Ok(NotarizeAuth::ApiKey {
                    key,
                    key_path: key_path.into(),
                    issuer,
                }),
                (Some(key), Some(issuer), Err(_)) => {
                    let mut api_key_file_name = OsString::from("AuthKey_");
                    api_key_file_name.push(&key);
                    api_key_file_name.push(".p8");
                    let mut key_path = None;

                    let mut search_paths = vec!["./private_keys".into()];
                    if let Some(home_dir) = dirs::home_dir() {
                        search_paths.push(home_dir.join("private_keys"));
                        search_paths.push(home_dir.join(".private_keys"));
                        search_paths.push(home_dir.join(".appstoreconnect/private_keys"));
                    }

                    for folder in search_paths {
                        if let Some(path) = find_api_key(folder, &api_key_file_name) {
                            key_path = Some(path);
                            break;
                        }
                    }

                    if let Some(key_path) = key_path {
                        Ok(NotarizeAuth::ApiKey {
                            key,
                            key_path,
                            issuer,
                        })
                    } else {
                        Err(Error::ApiKeyMissing {
                            filename: api_key_file_name
                                .into_string()
                                .expect("failed to convert api_key_file_name to string"),
                        })
                    }
                }
                _ => Err(Error::MissingNotarizeAuthVars),
            }
        }
    }
}

fn find_api_key(folder: PathBuf, file_name: &OsString) -> Option<PathBuf> {
    let path = folder.join(file_name);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}
