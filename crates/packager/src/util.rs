use cargo_packager_config::LogLevel;
use sha2::Digest;
use std::{
    fs::File,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Output,
};

use zip::ZipArchive;

pub fn display_path<P: AsRef<Path>>(p: P) -> String {
    dunce::simplified(&p.as_ref().components().collect::<PathBuf>())
        .display()
        .to_string()
}

/// Try to determine the current target triple.
///
/// Returns a target triple (e.g. `x86_64-unknown-linux-gnu` or `i686-pc-windows-msvc`) or an
/// `Error::Config` if the current config cannot be determined or is not some combination of the
/// following values:
/// `linux, mac, windows` -- `i686, x86, armv7` -- `gnu, musl, msvc`
///
/// * Errors:
///     * Unexpected system config
pub fn target_triple() -> crate::Result<String> {
    let arch = if cfg!(target_arch = "x86") {
        "i686"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "arm") {
        "armv7"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return Err(crate::Error::Architecture);
    };

    let os = if cfg!(target_os = "linux") {
        "unknown-linux"
    } else if cfg!(target_os = "macos") {
        "apple-darwin"
    } else if cfg!(target_os = "windows") {
        "pc-windows"
    } else if cfg!(target_os = "freebsd") {
        "unknown-freebsd"
    } else {
        return Err(crate::Error::Os);
    };

    let os = if cfg!(target_os = "macos") || cfg!(target_os = "freebsd") {
        String::from(os)
    } else {
        let env = if cfg!(target_env = "gnu") {
            "gnu"
        } else if cfg!(target_env = "musl") {
            "musl"
        } else if cfg!(target_env = "msvc") {
            "msvc"
        } else {
            return Err(crate::Error::Environment);
        };

        format!("{os}-{env}")
    };

    Ok(format!("{arch}-{os}"))
}

pub(crate) fn download(url: &str) -> crate::Result<Vec<u8>> {
    log::info!(action = "Downloading"; "{}", url);
    let response = ureq::get(url).call()?;
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub(crate) enum HashAlgorithm {
    #[cfg(target_os = "windows")]
    Sha256,
    Sha1,
}

/// Function used to download a file and checks SHA256 to verify the download.
pub(crate) fn download_and_verify(
    file: &str,
    url: &str,
    hash: &str,
    hash_algorithm: HashAlgorithm,
) -> crate::Result<Vec<u8>> {
    let data = download(url)?;
    log::info!(action = "Validating"; "{file} hash");

    match hash_algorithm {
        #[cfg(target_os = "windows")]
        HashAlgorithm::Sha256 => {
            let hasher = sha2::Sha256::new();
            verify(&data, hash, hasher)?;
        }
        HashAlgorithm::Sha1 => {
            let hasher = sha1::Sha1::new();
            verify(&data, hash, hasher)?;
        }
    }

    Ok(data)
}

fn verify(data: &Vec<u8>, hash: &str, mut hasher: impl Digest) -> crate::Result<()> {
    hasher.update(data);

    let url_hash = hasher.finalize().to_vec();
    let expected_hash = hex::decode(hash)?;
    if expected_hash == url_hash {
        Ok(())
    } else {
        Err(crate::Error::HashError)
    }
}

/// Extracts the zips from memory into a useable path.
pub(crate) fn extract_zip(data: &[u8], path: &Path) -> crate::Result<()> {
    let cursor = Cursor::new(data);

    let mut zipa = ZipArchive::new(cursor)?;

    for i in 0..zipa.len() {
        let mut file = zipa.by_index(i)?;

        if let Some(name) = file.enclosed_name() {
            let dest_path = path.join(name);
            if file.is_dir() {
                std::fs::create_dir_all(&dest_path)?;
                continue;
            }

            let parent = dest_path.parent().ok_or(crate::Error::ParentDirNotFound)?;

            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }

            let mut buff: Vec<u8> = Vec::new();
            file.read_to_end(&mut buff)?;

            let mut fileout = File::create(dest_path)?;
            fileout.write_all(&buff)?;
        }
    }

    Ok(())
}

pub(crate) enum Bitness {
    X86_32,
    X86_64,
    Unknown,
}

#[cfg(windows)]
pub(crate) fn os_bitness() -> crate::Result<Bitness> {
    use windows_sys::Win32::System::{
        Diagnostics::Debug::{PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_INTEL},
        SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO},
    };

    let mut system_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    unsafe { GetNativeSystemInfo(&mut system_info) };

    Ok(
        match unsafe { system_info.Anonymous.Anonymous.wProcessorArchitecture } {
            PROCESSOR_ARCHITECTURE_INTEL => Bitness::X86_32,
            PROCESSOR_ARCHITECTURE_AMD64 => Bitness::X86_64,
            _ => Bitness::Unknown,
        },
    )
}

pub(crate) fn log_if_needed(log_level: LogLevel, output: Output) {
    if output.status.success() && !output.stdout.is_empty() && log_level >= LogLevel::Debug {
        log::debug!(action = "stdout"; "{}", String::from_utf8_lossy(&output.stdout))
    } else if !output.status.success() && log_level >= LogLevel::Error {
        let action = if !output.stderr.is_empty() {
            "stderr"
        } else {
            "stdout"
        };
        let output = if !output.stderr.is_empty() {
            &output.stderr
        } else {
            &output.stdout
        };
        log::error!(action = action; "{}", String::from_utf8_lossy(output))
    }
}
