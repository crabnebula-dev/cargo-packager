#![cfg(windows)]

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use once_cell::sync::Lazy;
use winreg::{
    enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY},
    RegKey,
};

use crate::{
    config::{Config, ConfigExt},
    shell::CommandExt,
    util::{self, Bitness},
};

pub struct SignParams {
    pub product_name: String,
    pub digest_algorithm: String,
    pub certificate_thumbprint: String,
    pub timestamp_url: Option<String>,
    pub tsp: bool,
}

static SIGN_TOOL: Lazy<crate::Result<PathBuf>> = Lazy::new(|| {
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

fn locate_signtool() -> Option<PathBuf> {
    (*SIGN_TOOL).as_ref().ok().cloned()
}

pub fn sign_command(path: &str, params: &SignParams) -> crate::Result<(Command, PathBuf)> {
    // Construct SignTool command
    let signtool = locate_signtool().ok_or(crate::Error::SignToolNotFound)?;

    let mut cmd = Command::new(&signtool);
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

    cmd.arg(path);

    Ok((cmd, signtool))
}

pub(crate) trait ConfigSignExt {
    fn can_sign(&self) -> bool;
    fn sign_params(&self) -> SignParams;
}

impl ConfigSignExt for Config {
    fn can_sign(&self) -> bool {
        self.windows()
            .and_then(|w| w.certificate_thumbprint.as_ref())
            .is_some()
    }

    fn sign_params(&self) -> SignParams {
        let windows = self.windows();
        SignParams {
            product_name: self.product_name.clone(),
            digest_algorithm: windows
                .and_then(|w| w.digest_algorithm.as_ref())
                .as_ref()
                .map(|algorithm| algorithm.to_string())
                .unwrap_or_else(|| "sha256".to_string()),
            certificate_thumbprint: windows
                .and_then(|w| w.certificate_thumbprint.as_ref())
                .cloned()
                .unwrap_or_default(),
            timestamp_url: windows
                .and_then(|w| w.timestamp_url.as_ref())
                .as_ref()
                .map(|url| url.to_string()),
            tsp: windows.map(|w| w.tsp).unwrap_or_default(),
        }
    }
}

pub fn sign<P: AsRef<Path>>(path: P, params: &SignParams) -> crate::Result<()> {
    // Convert path to string reference, as we need to pass it as a command-line parameter to signtool
    let path_str = path.as_ref().to_str().unwrap();

    log::info!(action = "Signing"; "{} with identity \"{}\"", path_str, params.certificate_thumbprint);

    let (mut cmd, signtool) = sign_command(path_str, params)?;
    log::debug!("Running signtool {:?}", signtool);

    let output = cmd.output_ok()?;
    let stdout = String::from_utf8_lossy(output.stdout.as_slice()).into_owned();
    log::info!("{:?}", stdout);

    Ok(())
}

pub fn try_sign(
    file_path: &std::path::PathBuf,
    config: &crate::config::Config,
) -> crate::Result<()> {
    let windows_config = config.windows();
    if let Some(certificate_thumbprint) =
        windows_config.and_then(|c| c.certificate_thumbprint.as_ref())
    {
        log::info!(action = "Signing"; "{}", util::display_path(file_path));
        sign(
            file_path,
            &SignParams {
                product_name: config.product_name.clone(),
                digest_algorithm: windows_config
                    .and_then(|c| {
                        c.digest_algorithm
                            .as_ref()
                            .map(|algorithm| algorithm.to_string())
                    })
                    .unwrap_or_else(|| "sha256".to_string()),
                certificate_thumbprint: certificate_thumbprint.to_string(),
                timestamp_url: windows_config
                    .and_then(|c| c.timestamp_url.as_ref())
                    .map(|url| url.to_string()),
                tsp: windows_config.map(|c| c.tsp).unwrap_or_default(),
            },
        )?;
    }
    Ok(())
}
