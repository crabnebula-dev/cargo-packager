use error::Result;
use std::{env, path::PathBuf};

mod error;
mod starting_binary;

pub use error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageFormat {
    /// When no format is used (`cargo run`)
    None,
    /// The macOS application bundle (.app).
    App,
    /// The macOS DMG package (.dmg).
    Dmg,
    /// The Microsoft Software Installer (.msi) through WiX Toolset.
    Wix,
    /// The NSIS installer (.exe).
    Nsis,
    /// The Linux Debian package (.deb).
    Deb,
    /// The Linux AppImage package (.AppImage).
    AppImage,
}

/// Get the current package format.
/// Can only be used if the app was build with cargo-packager
/// and the `before-each-package-command` atribute.
#[cfg(feature = "auto-detect-formats")]
pub fn current_format() -> PackageFormat {
    // sync with PackageFormat::short_name function of packager crate
    // maybe having a special crate for the Config struct,
    // that both packager and resource-resolver could be a
    // better alternative
    if cfg!(CARGO_PACKAGER_FORMAT = "app") {
        PackageFormat::App
    } else if cfg!(CARGO_PACKAGER_FORMAT = "dmg") {
        PackageFormat::Dmg
    } else if cfg!(CARGO_PACKAGER_FORMAT = "wix") {
        PackageFormat::Wix
    } else if cfg!(CARGO_PACKAGER_FORMAT = "nsis") {
        PackageFormat::Nsis
    } else if cfg!(CARGO_PACKAGER_FORMAT = "deb") {
        PackageFormat::Deb
    } else if cfg!(CARGO_PACKAGER_FORMAT = "appimage") {
        PackageFormat::AppImage
    } else {
        PackageFormat::None
    }
}

/// Retrieves the currently running binary's path, taking into account security considerations.
///
/// The path is cached as soon as possible (before even `main` runs) and that value is returned
/// repeatedly instead of fetching the path every time. It is possible for the path to not be found,
/// or explicitly disabled (see following macOS specific behavior).
///
/// # Platform-specific behavior
///
/// On `macOS`, this function will return an error if the original path contained any symlinks
/// due to less protection on macOS regarding symlinks. This behavior can be disabled by setting the
/// `process-relaunch-dangerous-allow-symlink-macos` feature, although it is *highly discouraged*.
///
/// # Security
///
/// If the above platform-specific behavior does **not** take place, this function uses the
/// following resolution.
///
/// We canonicalize the path we received from [`std::env::current_exe`] to resolve any soft links.
/// This avoids the usual issue of needing the file to exist at the passed path because a valid
/// current executable result for our purpose should always exist. Notably,
/// [`std::env::current_exe`] also has a security section that goes over a theoretical attack using
/// hard links. Let's cover some specific topics that relate to different ways an attacker might
/// try to trick this function into returning the wrong binary path.
///
/// ## Symlinks ("Soft Links")
///
/// [`std::path::Path::canonicalize`] is used to resolve symbolic links to the original path,
/// including nested symbolic links (`link2 -> link1 -> bin`). On macOS, any results that include
/// a symlink are rejected by default due to lesser symlink protections. This can be disabled,
/// **although discouraged**, with the `process-relaunch-dangerous-allow-symlink-macos` feature.
///
/// ## Hard Links
///
/// A [Hard Link] is a named entry that points to a file in the file system.
/// On most systems, this is what you would think of as a "file". The term is
/// used on filesystems that allow multiple entries to point to the same file.
/// The linked [Hard Link] Wikipedia page provides a decent overview.
///
/// In short, unless the attacker was able to create the link with elevated
/// permissions, it should generally not be possible for them to hard link
/// to a file they do not have permissions to - with exception to possible
/// operating system exploits.
///
/// There are also some platform-specific information about this below.
///
/// ### Windows
///
/// Windows requires a permission to be set for the user to create a symlink
/// or a hard link, regardless of ownership status of the target. Elevated
/// permissions users have the ability to create them.
///
/// ### macOS
///
/// macOS allows for the creation of symlinks and hard links to any file.
/// Accessing through those links will fail if the user who owns the links
/// does not have the proper permissions on the original file.
///
/// ### Linux
///
/// Linux allows for the creation of symlinks to any file. Accessing the
/// symlink will fail if the user who owns the symlink does not have the
/// proper permissions on the original file.
///
/// Linux additionally provides a kernel hardening feature since version
/// 3.6 (30 September 2012). Most distributions since then have enabled
/// the protection (setting `fs.protected_hardlinks = 1`) by default, which
/// means that a vast majority of desktop Linux users should have it enabled.
/// **The feature prevents the creation of hardlinks that the user does not own
/// or have read/write access to.** [See the patch that enabled this].
///
/// [Hard Link]: https://en.wikipedia.org/wiki/Hard_link
/// [See the patch that enabled this]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=800179c9b8a1e796e441674776d11cd4c05d61d7
pub fn current_exe() -> Result<PathBuf> {
    starting_binary::STARTING_BINARY
        .cloned()
        .map_err(|e| Error::Io("Can't detect the path of the current exe".to_string(), e))
}

/// Retreive the resource path of your app, packaged with cargo packager.
/// This function behave the same as [`resource_dir`], except it accepts
/// a parameter that will be happened to the resource path when no packaging format
/// is used.
///
/// Example: You want to include the folder `crate/resource/icons/`.
///
/// - With `cargo run` command, you will have to execute
///     `resource_dir().unwrap().join("resource/icons/")` to get the path.
/// - With any other formats, it will be `resource_dir().unwrap().join("icons/")`.
///
/// ```
/// use cargo_packager_resource_resolver as resource_resolver;
/// use resource_resolver::{PackageFormat, resource_dir_with_suffix};
///
/// resource_dir_with_suffix(PackageFormat::None, "resource").unwrap().join("icons/");
/// ```
pub fn resource_dir_with_suffix(package_format: PackageFormat, suffix: &str) -> Result<PathBuf> {
    match package_format {
        PackageFormat::None => {
            let root_crate_dir = env::var("CARGO_MANIFEST_DIR")
                .map_err(|e| {
                    match e {
                        env::VarError::NotPresent => {
                            Error::Env("PackageFormat::None was use, but CARGO_MANIFEST_DIR environnement variable was not defined".to_string())
                        },
                        _ => Error::Var("Can't access CARGO_MANIFEST_DIR environnement variable".to_string(), e)
                    }
                })?;
            Ok(PathBuf::from(root_crate_dir).join(suffix))
        }
        PackageFormat::App | PackageFormat::Dmg => {
            let exe = current_exe()?;
            let exe_dir = exe.parent().unwrap();
            exe_dir
                .join("../Resources")
                .canonicalize()
                .map_err(|e| Error::Io("".to_string(), e))
        }
        PackageFormat::Wix => Err(Error::UnsupportedPlatform),
        PackageFormat::Nsis => {
            let exe = current_exe()?;
            let exe_dir = exe.parent().unwrap();
            Ok(exe_dir.to_path_buf())
        }
        PackageFormat::Deb => {
            // maybe this is not reliable, and we need to get the app name from argument
            let exe = current_exe()?;
            let binary_name = exe.file_name().unwrap().to_string_lossy();

            let path = format!("/usr/lib/{}/", binary_name);
            return Ok(PathBuf::from(path));
        }
        PackageFormat::AppImage => todo!(),
    }
}

/// Retreive the resource path of your app, packaged with cargo packager.
#[inline]
pub fn resource_dir(package_format: PackageFormat) -> Result<PathBuf> {
    resource_dir_with_suffix(package_format, "")
}
