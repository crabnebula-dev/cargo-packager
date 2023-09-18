use sha2::Digest;
use std::{
    fs::File,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use zip::ZipArchive;

use crate::shell::CommandExt;

pub fn display_path<P: AsRef<Path>>(p: P) -> String {
    dunce::simplified(&p.as_ref().components().collect::<PathBuf>())
        .display()
        .to_string()
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
            log:: warn!(
                "failed to determine target arch using rustc, error: `{}`. The fallback is the architecture of the machine that compiled this crate.",
                err,
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
    log::info!(action = "Downloading"; "{}", url);
    let response = ureq::get(url).call().map_err(Box::new)?;
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

#[cfg(windows)]
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

/// Returns true if the path has a filename indicating that it is a high-density
/// "retina" icon.  Specifically, returns true the file stem ends with
/// "@2x" (a convention specified by the [Apple developer docs](
/// https://developer.apple.com/library/mac/documentation/GraphicsAnimation/Conceptual/HighResolutionOSX/Optimizing/Optimizing.html)).
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "macos",
))]
pub(crate) fn is_retina<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .map(|stem| stem.ends_with("@2x"))
        .unwrap_or(false)
}

/// Creates a new file at the given path, creating any parent directories as needed.
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) fn create_file(path: &Path) -> crate::Result<std::io::BufWriter<File>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    Ok(std::io::BufWriter::new(file))
}

// Given a list of icon files, try to produce an ICNS file in the out_dir
// and return the path to it.  Returns `Ok(None)` if no usable icons
// were provided.
#[cfg(target_os = "macos")]
pub fn create_icns_file(out_dir: &Path, config: &crate::Config) -> crate::Result<Option<PathBuf>> {
    use image::GenericImageView;

    if config.icons.as_ref().map(|i| i.len()).unwrap_or_default() == 0 {
        return Ok(None);
    }

    // If one of the icon files is already an ICNS file, just use that.
    if let Some(icons) = &config.icons {
        std::fs::create_dir_all(out_dir)?;

        for icon_path in icons {
            let icon_path = PathBuf::from(icon_path);
            if icon_path.extension() == Some(std::ffi::OsStr::new("icns")) {
                let dest_path =
                    out_dir.join(icon_path.file_name().expect("could not get icon filename"));
                std::fs::copy(&icon_path, &dest_path)?;

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
    if let Some(icons) = &config.icons {
        for icon_path in icons {
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
        std::fs::create_dir_all(out_dir)?;
        let mut dest_path = out_dir.to_path_buf();
        dest_path.push(config.product_name.clone());
        dest_path.set_extension("icns");
        let icns_file = std::io::BufWriter::new(File::create(&dest_path)?);
        family.write(icns_file)?;
        Ok(Some(dest_path))
    } else {
        Err(crate::Error::InvalidIconList)
    }
}

// Converts an image::DynamicImage into an icns::Image.
#[cfg(target_os = "macos")]
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
