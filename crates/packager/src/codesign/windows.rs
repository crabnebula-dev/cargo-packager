// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{fmt::Debug, path::Path, process::Command};

#[cfg(windows)]
use once_cell::sync::Lazy;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use winreg::{
    enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY},
    RegKey,
};

use crate::{config::Config, shell::CommandExt, util};

#[cfg(windows)]
use crate::util::Bitness;

#[derive(Debug)]
pub struct SignParams {
    pub product_name: String,
    pub digest_algorithm: String,
    pub certificate_thumbprint: String,
    pub timestamp_url: Option<String>,
    pub tsp: bool,
    pub sign_command: Option<String>,
}

pub(crate) trait ConfigSignExt {
    fn can_sign(&self) -> bool;
    fn custom_sign_command(&self) -> bool;
    fn sign_params(&self) -> SignParams;
}

impl ConfigSignExt for Config {
    fn can_sign(&self) -> bool {
        self.windows()
            .and_then(|w| w.certificate_thumbprint.as_ref())
            .is_some()
            || self.custom_sign_command()
    }

    fn custom_sign_command(&self) -> bool {
        self.windows()
            .and_then(|w| w.sign_command.as_ref())
            .is_some()
    }

    fn sign_params(&self) -> SignParams {
        let windows = self.windows();
        SignParams {
            product_name: self.product_name.clone(),
            digest_algorithm: windows
                .and_then(|w| w.digest_algorithm.as_ref())
                .cloned()
                .unwrap_or_else(|| "sha256".to_string()),
            certificate_thumbprint: windows
                .and_then(|w| w.certificate_thumbprint.as_ref())
                .cloned()
                .unwrap_or_default(),
            timestamp_url: windows.and_then(|w| w.timestamp_url.as_ref()).cloned(),
            tsp: windows.map(|w| w.tsp).unwrap_or_default(),
            sign_command: windows.and_then(|w| w.sign_command.as_ref()).cloned(),
        }
    }
}

#[cfg(windows)]
static SIGN_TOOL: Lazy<crate::Result<PathBuf>> = Lazy::new(|| {
    let _s = tracing::span!(tracing::Level::TRACE, "locate_signtool");
    const INSTALLED_ROOTS_REGKEY_PATH: &str = r"SOFTWARE\Microsoft\Windows Kits\Installed Roots";
    const KITS_ROOT_REGVALUE_NAME: &str = r"KitsRoot10";

    let installed_roots_key_path = Path::new(INSTALLED_ROOTS_REGKEY_PATH);

    // Open 32-bit HKLM "Installed Roots" key
    let installed_roots_key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(installed_roots_key_path, KEY_READ | KEY_WOW64_32KEY)?;
    // Get the Windows SDK root path
    let kits_root_10_path: String = installed_roots_key.get_value(KITS_ROOT_REGVALUE_NAME)?;
    // Construct Windows SDK bin path
    let kits_root_10_bin_path = Path::new(&kits_root_10_path).join("bin");

    let mut installed_kits: Vec<String> = installed_roots_key
        .enum_keys()
        /* Report and ignore errors, pass on values. */
        .filter_map(|res| match res {
            Ok(v) => Some(v),
            Err(_) => None,
        })
        .collect();

    // Sort installed kits
    installed_kits.sort();

    /* Iterate through installed kit version keys in reverse (from newest to oldest),
    adding their bin paths to the list.
    Windows SDK 10 v10.0.15063.468 and later will have their signtools located there. */
    let mut kit_bin_paths: Vec<PathBuf> = installed_kits
        .iter()
        .rev()
        .map(|kit| kits_root_10_bin_path.join(kit))
        .collect();

    /* Add kits root bin path.
    For Windows SDK 10 versions earlier than v10.0.15063.468, signtool will be located there. */
    kit_bin_paths.push(kits_root_10_bin_path);

    // Choose which version of SignTool to use based on OS bitness
    let arch_dir = match util::os_bitness().expect("failed to get os bitness") {
        Bitness::X86_32 => "x86",
        Bitness::X86_64 => "x64",
        _ => return Err(crate::Error::UnsupportedBitness),
    };

    /* Iterate through all bin paths, checking for existence of a SignTool executable. */
    for kit_bin_path in &kit_bin_paths {
        /* Construct SignTool path. */
        let signtool_path = kit_bin_path.join(arch_dir).join("signtool.exe");

        /* Check if SignTool exists at this location. */
        if signtool_path.exists() {
            // SignTool found. Return it.
            return Ok(signtool_path);
        }
    }

    Err(crate::Error::SignToolNotFound)
});

#[cfg(windows)]
fn signtool() -> Option<PathBuf> {
    (*SIGN_TOOL).as_ref().ok().cloned()
}

#[tracing::instrument(level = "trace")]
pub fn sign_command_custom<P: AsRef<Path> + Debug>(
    path: P,
    command: &str,
) -> crate::Result<Command> {
    let mut args = command.trim().split(' ');

    let bin = args
        .next()
        .expect("custom signing command doesn't contain a bin?");

    let mut cmd = Command::new(bin);

    for arg in args {
        if arg == "%1" {
            cmd.arg(path.as_ref());
        } else {
            cmd.arg(arg);
        }
    }

    Ok(cmd)
}

#[cfg(windows)]
#[tracing::instrument(level = "trace")]
pub fn sign_command_default<P: AsRef<Path> + Debug>(
    path: P,
    params: &SignParams,
) -> crate::Result<Command> {
    let signtool = signtool().ok_or(crate::Error::SignToolNotFound)?;

    let mut cmd = Command::new(signtool);
    cmd.arg("sign");
    cmd.args(["/fd", &params.digest_algorithm]);
    cmd.args(["/sha1", &params.certificate_thumbprint]);
    cmd.args(["/d", &params.product_name]);

    if let Some(ref timestamp_url) = params.timestamp_url {
        if params.tsp {
            cmd.args(["/tr", timestamp_url]);
            cmd.args(["/td", &params.digest_algorithm]);
        } else {
            cmd.args(["/t", timestamp_url]);
        }
    }

    cmd.arg(path.as_ref());

    Ok(cmd)
}

#[tracing::instrument(level = "trace")]
pub fn sign_command<P: AsRef<Path> + Debug>(
    path: P,
    params: &SignParams,
) -> crate::Result<Command> {
    match &params.sign_command {
        Some(custom_command) => sign_command_custom(path, custom_command),
        #[cfg(windows)]
        None => sign_command_default(path, params),

        // should not be reachable
        #[cfg(not(windows))]
        None => Ok(Command::new("")),
    }
}

#[tracing::instrument(level = "trace")]
pub fn sign_custom<P: AsRef<Path> + Debug>(path: P, custom_command: &str) -> crate::Result<()> {
    let path = path.as_ref();

    tracing::info!(
        "Codesigning {} with a custom signing command",
        util::display_path(path),
    );

    let mut cmd = sign_command_custom(path, custom_command)?;

    let output = cmd
        .output_ok()
        .map_err(crate::Error::CustomSignCommandFailed)?;

    let stdout = String::from_utf8_lossy(output.stdout.as_slice());
    tracing::info!("{:?}", stdout);

    Ok(())
}

#[tracing::instrument(level = "trace")]
#[cfg(windows)]
pub fn sign_default<P: AsRef<Path> + Debug>(path: P, params: &SignParams) -> crate::Result<()> {
    let signtool = signtool().ok_or(crate::Error::SignToolNotFound)?;
    let path = path.as_ref();

    tracing::info!(
        "Codesigning {} with identity \"{}\"",
        util::display_path(path),
        params.certificate_thumbprint
    );

    let mut cmd = sign_command_default(path, params)?;

    tracing::debug!("Running signtool {:?}", signtool);
    let output = cmd.output_ok().map_err(crate::Error::SignToolFailed)?;

    let stdout = String::from_utf8_lossy(output.stdout.as_slice());
    tracing::info!("{:?}", stdout);

    Ok(())
}

#[tracing::instrument(level = "trace")]
pub fn sign<P: AsRef<Path> + Debug>(path: P, params: &SignParams) -> crate::Result<()> {
    match &params.sign_command {
        Some(custom_command) => sign_custom(path, custom_command),
        #[cfg(windows)]
        None => sign_default(path, params),

        // should not be reachable
        #[cfg(not(windows))]
        None => Ok(()),
    }
}

#[tracing::instrument(level = "trace")]
pub fn try_sign(
    file_path: &std::path::PathBuf,
    config: &crate::config::Config,
) -> crate::Result<()> {
    if config.can_sign() {
        sign(file_path, &config.sign_params())?;
    }
    Ok(())
}
