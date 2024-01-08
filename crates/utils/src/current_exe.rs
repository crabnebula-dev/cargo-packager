// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

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
    STARTING_BINARY.cloned()
}

use ctor::ctor;
use std::{
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
};

/// A cached version of the current binary using [`ctor`] to cache it before even `main` runs.
#[ctor]
#[used]
static STARTING_BINARY: StartingBinary = StartingBinary::new();

/// Represents a binary path that was cached when the program was loaded.
struct StartingBinary(std::io::Result<PathBuf>);

impl StartingBinary {
    /// Find the starting executable as safely as possible.
    fn new() -> Self {
        // see notes on current_exe() for security implications
        let dangerous_path = match std::env::current_exe() {
            Ok(dangerous_path) => dangerous_path,
            error @ Err(_) => return Self(error),
        };

        // note: this only checks symlinks on problematic platforms, see implementation below
        if let Some(symlink) = Self::has_symlink(&dangerous_path) {
            return Self(Err(Error::new(
        ErrorKind::InvalidData,
        format!("StartingBinary found current_exe() that contains a symlink on a non-allowed platform: {}", symlink.display()),
      )));
        }

        // we canonicalize the path to resolve any symlinks to the real exe path
        Self(dangerous_path.canonicalize())
    }

    /// A clone of the [`PathBuf`] found to be the starting path.
    ///
    /// Because [`Error`] is not clone-able, it is recreated instead.
    pub(super) fn cloned(&self) -> Result<PathBuf> {
        self.0
            .as_ref()
            .map(Clone::clone)
            .map_err(|e| Error::new(e.kind(), e.to_string()))
    }

    /// We only care about checking this on macOS currently, as it has the least symlink protections.
    #[cfg(any(
        not(target_os = "macos"),
        feature = "process-relaunch-dangerous-allow-symlink-macos"
    ))]
    fn has_symlink(_: &Path) -> Option<&Path> {
        None
    }

    /// We only care about checking this on macOS currently, as it has the least symlink protections.
    #[cfg(all(
        target_os = "macos",
        not(feature = "process-relaunch-dangerous-allow-symlink-macos")
    ))]
    fn has_symlink(path: &Path) -> Option<&Path> {
        path.ancestors().find(|ancestor| {
            matches!(
                ancestor
                    .symlink_metadata()
                    .as_ref()
                    .map(std::fs::Metadata::file_type)
                    .as_ref()
                    .map(std::fs::FileType::is_symlink),
                Ok(true)
            )
        })
    }
}
