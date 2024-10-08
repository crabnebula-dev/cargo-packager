// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use sha2::Digest;
use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use zip::ZipArchive;

use crate::{shell::CommandExt, Error};

#[inline]
pub(crate) fn cross_command(script: &str) -> Command {
    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    cmd.arg("/S").arg("/C").arg(script);
    #[cfg(not(windows))]
    let mut cmd = Command::new("sh");
    cmd.current_dir(dunce::canonicalize(std::env::current_dir().unwrap()).unwrap());
    #[cfg(not(windows))]
    cmd.arg("-c").arg(script);
    cmd
}

#[inline]
pub fn display_path<P: AsRef<Path>>(p: P) -> String {
    dunce::simplified(&p.as_ref().components().collect::<PathBuf>())
        .display()
        .to_string()
}

/// Recursively create a directory and all of its parent components if they
/// are missing after Deleting the existing directory (if it exists).
#[inline]
pub fn create_clean_dir<P: AsRef<Path>>(path: P) -> crate::Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?;
    }
    fs::create_dir_all(path).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))
}

/// Creates a new file at the given path, creating any parent directories as needed.
#[inline]
pub(crate) fn create_file(path: &Path) -> crate::Result<std::io::BufWriter<File>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?;
    }
    let file = File::create(path).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?;
    Ok(std::io::BufWriter::new(file))
}

#[derive(Debug, PartialEq, Eq)]
struct RustCfg {
    target_arch: Option<String>,
}

fn parse_rust_cfg(cfg: String) -> RustCfg {
    let target_line = "target_arch=\"";
    let mut target_arch = None;
    for line in cfg.split('\n') {
        if line.starts_with(target_line) {
            let len = target_line.len();
            let arch = line.chars().skip(len).take(line.len() - len - 1).collect();
            target_arch.replace(arch);
        }
    }
    RustCfg { target_arch }
}

/// Try to determine the current target triple.
///
/// Returns a target triple (e.g. `x86_64-unknown-linux-gnu` or `i686-pc-windows-msvc`) or an
/// error if the current config cannot be determined or is not some combination of the
/// following values:
/// `linux, mac, windows` -- `i686, x86, armv7` -- `gnu, musl, msvc`
pub fn target_triple() -> crate::Result<String> {
    let arch_res = Command::new("rustc").args(["--print", "cfg"]).output_ok();

    let arch = match arch_res {
        Ok(output) => parse_rust_cfg(String::from_utf8_lossy(&output.stdout).into())
            .target_arch
            .expect("could not find `target_arch` when running `rustc --print cfg`."),
        Err(err) => {
            tracing:: debug!(
                "Failed to determine target arch using rustc, error: `{err}`. Falling back to the architecture of the machine that compiled this crate.",
            );
            if cfg!(target_arch = "x86") {
                "i686".into()
            } else if cfg!(target_arch = "x86_64") {
                "x86_64".into()
            } else if cfg!(target_arch = "arm") {
                "armv7".into()
            } else if cfg!(target_arch = "aarch64") {
                "aarch64".into()
            } else {
                return Err(crate::Error::Architecture);
            }
        }
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
    tracing::debug!("Downloading {}", url);

    // This is required because ureq does not bind native-tls as the default TLS implementation when rustls is not available.
    // See <https://github.com/crabnebula-dev/cargo-packager/issues/127>
    #[cfg(feature = "native-tls")]
    let agent = ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(
            native_tls::TlsConnector::new().unwrap(),
        ))
        .try_proxy_from_env(true)
        .build();
    #[cfg(not(feature = "native-tls"))]
    let agent = ureq::AgentBuilder::new().try_proxy_from_env(true).build();

    let response = agent.get(url).call().map_err(Box::new)?;
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[derive(Clone, Copy)]
pub(crate) enum HashAlgorithm {
    #[cfg(target_os = "windows")]
    Sha256,
    Sha1,
}

/// Function used to download a file and checks SHA256 to verify the download.
pub(crate) fn download_and_verify<P: AsRef<Path>>(
    path: P,
    url: &str,
    hash: &str,
    hash_algorithm: HashAlgorithm,
) -> crate::Result<Vec<u8>> {
    let data = download(url)?;
    tracing::debug!("Validating {} hash", path.as_ref().display());
    verify_hash(&data, hash, hash_algorithm)?;
    Ok(data)
}

pub(crate) fn verify_hash(
    data: &[u8],
    hash: &str,
    hash_algorithm: HashAlgorithm,
) -> crate::Result<()> {
    match hash_algorithm {
        #[cfg(target_os = "windows")]
        HashAlgorithm::Sha256 => {
            let hasher = sha2::Sha256::new();
            verify_data_with_hasher(data, hash, hasher)
        }
        HashAlgorithm::Sha1 => {
            let hasher = sha1::Sha1::new();
            verify_data_with_hasher(data, hash, hasher)
        }
    }
}

fn verify_data_with_hasher(data: &[u8], hash: &str, mut hasher: impl Digest) -> crate::Result<()> {
    hasher.update(data);

    let url_hash = hasher.finalize().to_vec();
    let expected_hash = hex::decode(hash)?;
    if expected_hash == url_hash {
        Ok(())
    } else {
        Err(crate::Error::HashError)
    }
}

pub(crate) fn verify_file_hash<P: AsRef<Path>>(
    path: P,
    hash: &str,
    hash_algorithm: HashAlgorithm,
) -> crate::Result<()> {
    let data = fs::read(&path).map_err(|e| Error::IoWithPath(path.as_ref().to_path_buf(), e))?;
    verify_hash(&data, hash, hash_algorithm)
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
                fs::create_dir_all(&dest_path)
                    .map_err(|e| Error::IoWithPath(dest_path.clone(), e))?;
                continue;
            }

            let parent = dest_path
                .parent()
                .ok_or_else(|| crate::Error::ParentDirNotFound(dest_path.clone()))?;

            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| Error::IoWithPath(parent.to_path_buf(), e))?
            }

            let mut buff: Vec<u8> = Vec::new();
            file.read_to_end(&mut buff)?;

            let mut fileout = File::create(dest_path)?;
            fileout.write_all(&buff)?;
        }
    }

    Ok(())
}

#[cfg(windows)]
pub(crate) enum Bitness {
    X86_32,
    X86_64,
    Unknown,
}

#[cfg(windows)]
pub(crate) fn os_bitness() -> crate::Result<Bitness> {
    use windows_sys::Win32::System::{
        SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO},
        SystemInformation::{PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_INTEL},
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

/// Returns true if the path has a filename indicating that it is a high-density
/// "retina" icon.  Specifically, returns true the file stem ends with
/// "@2x" (a convention specified by the [Apple developer docs](
/// https://developer.apple.com/library/mac/documentation/GraphicsAnimation/Conceptual/HighResolutionOSX/Optimizing/Optimizing.html)).xw
pub(crate) fn is_retina<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .map(|stem| stem.ends_with("@2x"))
        .unwrap_or(false)
}

// Given a list of icon files, try to produce an ICNS file in the out_dir
// and return the path to it.  Returns `Ok(None)` if no usable icons
// were provided.
pub fn create_icns_file(out_dir: &Path, config: &crate::Config) -> crate::Result<Option<PathBuf>> {
    use image::GenericImageView;

    let icons = config.icons()?;
    if icons.as_ref().map(|i| i.len()).unwrap_or_default() == 0 {
        return Ok(None);
    }

    // If one of the icon files is already an ICNS file, just use that.
    if let Some(icons) = icons {
        fs::create_dir_all(out_dir).map_err(|e| Error::IoWithPath(out_dir.to_path_buf(), e))?;
        for icon_path in icons {
            if icon_path.extension() == Some(std::ffi::OsStr::new("icns")) {
                let dest_path = out_dir.join(
                    icon_path
                        .file_name()
                        .ok_or_else(|| crate::Error::FailedToExtractFilename(icon_path.clone()))?,
                );
                fs::copy(&icon_path, &dest_path)
                    .map_err(|e| Error::CopyFile(icon_path.clone(), dest_path.clone(), e))?;

                return Ok(Some(dest_path));
            }
        }
    }

    // Otherwise, read available images and pack them into a new ICNS file.
    let mut family = icns::IconFamily::new();

    #[inline]
    fn add_icon_to_family(
        icon: image::DynamicImage,
        density: u32,
        family: &mut icns::IconFamily,
    ) -> std::io::Result<()> {
        // Try to add this image to the icon family.  Ignore images whose sizes
        // don't map to any ICNS icon type; print warnings and skip images that
        // fail to encode.
        match icns::IconType::from_pixel_size_and_density(icon.width(), icon.height(), density) {
            Some(icon_type) => {
                if !family.has_icon_with_type(icon_type) {
                    let icon = make_icns_image(icon)?;
                    family.add_icon_with_type(&icon, icon_type)?;
                }
                Ok(())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No matching IconType",
            )),
        }
    }

    let mut images_to_resize: Vec<(image::DynamicImage, u32, u32)> = vec![];
    if let Some(icons) = config.icons()? {
        for icon_path in &icons {
            let icon = image::open(icon_path)?;
            let density = if is_retina(icon_path) { 2 } else { 1 };
            let (w, h) = icon.dimensions();
            let orig_size = std::cmp::min(w, h);
            let next_size_down = 2f32.powf((orig_size as f32).log2().floor()) as u32;
            if orig_size > next_size_down {
                images_to_resize.push((icon, next_size_down, density));
            } else {
                add_icon_to_family(icon, density, &mut family)?;
            }
        }
    }

    for (icon, next_size_down, density) in images_to_resize {
        let icon = icon.resize_exact(
            next_size_down,
            next_size_down,
            image::imageops::FilterType::Lanczos3,
        );
        add_icon_to_family(icon, density, &mut family)?;
    }

    if !family.is_empty() {
        fs::create_dir_all(out_dir).map_err(|e| Error::IoWithPath(out_dir.to_path_buf(), e))?;
        let mut dest_path = out_dir.to_path_buf();
        dest_path.push(config.product_name.clone());
        dest_path.set_extension("icns");
        let file =
            File::create(&dest_path).map_err(|e| Error::IoWithPath(out_dir.to_path_buf(), e))?;
        let icns_file = std::io::BufWriter::new(file);
        family.write(icns_file)?;
        Ok(Some(dest_path))
    } else {
        Err(crate::Error::InvalidIconList)
    }
}

// Converts an image::DynamicImage into an icns::Image.
fn make_icns_image(img: image::DynamicImage) -> std::io::Result<icns::Image> {
    let pixel_format = match img.color() {
        image::ColorType::Rgba8 => icns::PixelFormat::RGBA,
        image::ColorType::Rgb8 => icns::PixelFormat::RGB,
        image::ColorType::La8 => icns::PixelFormat::GrayAlpha,
        image::ColorType::L8 => icns::PixelFormat::Gray,
        _ => {
            let msg = format!("unsupported ColorType: {:?}", img.color());
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, msg));
        }
    };
    icns::Image::from_data(pixel_format, img.width(), img.height(), img.into_bytes())
}

/// Writes a tar file to the given writer containing the given directory.
///
/// The generated tar contains the `src_dir` as a whole and not just its files,
/// so if we are creating a tar for:
/// ```text
/// dir/
///   |_ file1
///   |_ file2
///   |_ file3
/// ```
/// the generated tar will contain the following entries:
/// ```text
/// - dir
/// - dir/file1
/// - dir/file2
/// - dir/file3
/// ```
pub fn create_tar_from_dir<P: AsRef<Path>, W: Write>(src_dir: P, dest_file: W) -> crate::Result<W> {
    let src_dir = src_dir.as_ref();
    let filename = src_dir
        .file_name()
        .ok_or_else(|| crate::Error::FailedToExtractFilename(src_dir.to_path_buf()))?;
    let mut builder = tar::Builder::new(dest_file);
    builder.follow_symlinks(false);
    builder.append_dir_all(filename, src_dir)?;
    builder.into_inner().map_err(Into::into)
}

pub trait PathExt {
    fn with_additional_extension(&self, extension: impl AsRef<OsStr>) -> PathBuf;
}

impl PathExt for Path {
    fn with_additional_extension(&self, extension: impl AsRef<OsStr>) -> PathBuf {
        match self.extension() {
            Some(ext) => {
                let mut e = ext.to_os_string();
                e.push(".");
                e.push(extension);
                self.with_extension(e)
            }
            None => self.with_extension(extension),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_appends_ext() {
        // Something that has an extention getting another suffix.
        assert_eq!(
            PathBuf::from("./asset.zip").with_additional_extension("sig"),
            PathBuf::from("./asset.zip.sig")
        );

        // Something that doesn't have an extention, setting its extension.
        assert_eq!(
            PathBuf::from("./executable").with_additional_extension("sig"),
            PathBuf::from("./executable.sig")
        )
    }
}
