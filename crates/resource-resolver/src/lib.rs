use std::{env, path::PathBuf};

pub mod error;
pub mod starting_binary;

use error::Result;

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

impl PackageFormat {
    /// Get the current package format
    pub fn get_current() -> Self {
        // sync with PackageFormat::short_name function of packager crate
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
pub fn current_exe() -> std::io::Result<PathBuf> {
    starting_binary::STARTING_BINARY.cloned()
}

/// See [`resource_dir`] for the general explanation. This function behave the same except
/// it accepts a parameter that will be happened to the resource path when no packaging format
/// is used.
pub fn resource_dir_with_suffix(suffix: &str) -> Result<PathBuf> {
    #[cfg(any(CARGO_PACKAGER_FORMAT = "app", CARGO_PACKAGER_FORMAT = "dmg"))]
    {
        let exe = current_exe()?;
        let exe_dir = exe.parent().expect("failed to get exe directory");
        return exe_dir
            .join("../Resources")
            .canonicalize()
            .map_err(Into::into);
    }

    #[cfg(CARGO_PACKAGER_FORMAT = "wix")]
    {
        return Err(Error::UnsupportedPlatform);
    }

    #[cfg(CARGO_PACKAGER_FORMAT = "nsis")]
    {
        let exe = current_exe()?;
        let exe_dir = exe.parent().expect("failed to get exe directory");
        return Ok(exe_dir.to_path_buf());
    }

    #[cfg(CARGO_PACKAGER_FORMAT = "deb")]
    {
        let binary_name = env!("CARGO_PACKAGER_MAIN_BINARY_NAME");
        let path = format!("/usr/lib/{}/", binary_name);
        return Ok(PathBuf::from(path));
    }

    #[cfg(CARGO_PACKAGER_FORMAT = "appimage")]
    {
        return Err(Error::UnsupportedPlatform);
    }

    // when cargo run
    let root_crate_dir = env::var("CARGO_MANIFEST_DIR")?;
    Ok(PathBuf::from(root_crate_dir).join(suffix))
}

/// To use this function, you have to build your package with
/// the `before-each-package-command` atribute.
///
/// Warning: Having resource folders inside folders can create inconsistency.
///
/// Example: You want to include the folder `crate/resource/icons/`.
///
/// - With `cargo run` command, you will have to execute
///     `resource_dir().unwrap().join("resource/icons/")` to get the path.
/// - With any other formats, it will be `resource_dir().unwrap().join("icons/")`.
///
/// For this use case, you can use [`self::resource_dir_with_suffix`]
/// ```
/// use cargo_packager_resource_resolver::resource_dir_with_suffix;
///
/// resource_dir_with_suffix("resource").unwrap().join("icons/");
/// ```
///
pub fn resource_dir() -> Result<PathBuf> {
    resource_dir_with_suffix("")
}
