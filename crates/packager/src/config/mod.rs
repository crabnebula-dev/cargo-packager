// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Configuration type and associated utilities.

use std::{
    collections::HashMap,
    ffi::OsString,
    fmt::{self, Display},
    path::{Path, PathBuf},
};

use relative_path::PathExt;
use serde::{Deserialize, Serialize};

use crate::util;

mod builder;
mod category;

pub use builder::*;
pub use category::AppCategory;

pub use cargo_packager_utils::PackageFormat;

/// **macOS-only**. Corresponds to CFBundleTypeRole
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum BundleTypeRole {
    /// CFBundleTypeRole.Editor. Files can be read and edited.
    Editor,
    /// CFBundleTypeRole.Viewer. Files can be read.
    Viewer,
    /// CFBundleTypeRole.Shell
    Shell,
    /// CFBundleTypeRole.QLGenerator
    QLGenerator,
    /// CFBundleTypeRole.None
    None,
}

impl Default for BundleTypeRole {
    fn default() -> Self {
        Self::Editor
    }
}

impl Display for BundleTypeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Editor => write!(f, "Editor"),
            Self::Viewer => write!(f, "Viewer"),
            Self::Shell => write!(f, "Shell"),
            Self::QLGenerator => write!(f, "QLGenerator"),
            Self::None => write!(f, "None"),
        }
    }
}

/// A file association configuration.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct FileAssociation {
    /// File extensions to associate with this app. e.g. 'png'
    pub extensions: Vec<String>,
    /// The mime-type e.g. 'image/png' or 'text/plain'. **Linux-only**.
    #[serde(alias = "mime-type", alias = "mime_type")]
    pub mime_type: Option<String>,
    /// The association description. **Windows-only**. It is displayed on the `Type` column on Windows Explorer.
    pub description: Option<String>,
    /// The name. Maps to `CFBundleTypeName` on macOS. Defaults to the first item in `ext`
    pub name: Option<String>,
    /// The app’s role with respect to the type. Maps to `CFBundleTypeRole` on macOS.
    /// Defaults to [`BundleTypeRole::Editor`]
    #[serde(default)]
    pub role: BundleTypeRole,
}

impl FileAssociation {
    /// Creates a new [`FileAssociation`] using provided extensions.
    pub fn new<I, S>(extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            extensions: extensions.into_iter().map(Into::into).collect(),
            mime_type: None,
            description: None,
            name: None,
            role: BundleTypeRole::default(),
        }
    }

    /// Set the extenstions to associate with this app. e.g. 'png'.
    pub fn extensions<I, S>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extensions = extensions.into_iter().map(Into::into).collect();
        self
    }

    /// Set the mime-type e.g. 'image/png' or 'text/plain'. **Linux-only**.
    pub fn mime_type<S: Into<String>>(mut self, mime_type: S) -> Self {
        self.mime_type.replace(mime_type.into());
        self
    }

    /// Se the association description. **Windows-only**. It is displayed on the `Type` column on Windows Explorer.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description.replace(description.into());
        self
    }

    /// Set he name. Maps to `CFBundleTypeName` on macOS. Defaults to the first item in `ext`
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name.replace(name.into());
        self
    }

    /// Set he app’s role with respect to the type. Maps to `CFBundleTypeRole` on macOS.
    /// Defaults to [`BundleTypeRole::Editor`]
    pub fn role(mut self, role: BundleTypeRole) -> Self {
        self.role = role;
        self
    }
}

/// Deep link protocol
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct DeepLinkProtocol {
    /// URL schemes to associate with this app without `://`. For example `my-app`
    pub schemes: Vec<String>,
    /// The protocol name. **macOS-only** and maps to `CFBundleTypeName`. Defaults to `<bundle-id>.<schemes[0]>`
    pub name: Option<String>,
    /// The app's role for these schemes. **macOS-only** and maps to `CFBundleTypeRole`.
    #[serde(default)]
    pub role: BundleTypeRole,
}

impl DeepLinkProtocol {
    /// Creates a new [`DeepLinkProtocol``] using provided schemes.
    pub fn new<I, S>(schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            schemes: schemes.into_iter().map(Into::into).collect(),
            name: None,
            role: BundleTypeRole::default(),
        }
    }

    /// Set he name. Maps to `CFBundleTypeName` on macOS. Defaults to the first item in `ext`
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name.replace(name.into());
        self
    }

    /// Set he app’s role with respect to the type. Maps to `CFBundleTypeRole` on macOS.
    /// Defaults to [`BundleTypeRole::Editor`]
    pub fn role(mut self, role: BundleTypeRole) -> Self {
        self.role = role;
        self
    }
}

/// The Linux Debian configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct DebianConfig {
    /// The list of Debian dependencies.
    pub depends: Option<Dependencies>,
    /// Path to a custom desktop file Handlebars template.
    ///
    /// Available variables: `categories`, `comment` (optional), `exec`, `icon` and `name`.
    ///
    /// Default file contents:
    /// ```text
    /// [Desktop Entry]
    /// Categories={{categories}}
    /// {{#if comment}}
    /// Comment={{comment}}
    /// {{/if}}
    /// Exec={{exec}}
    /// Icon={{icon}}
    /// Name={{name}}
    /// Terminal=false
    /// Type=Application
    /// {{#if mime_type}}
    /// MimeType={{mime_type}}
    /// {{/if}}
    /// ```
    #[serde(alias = "desktop-template", alias = "desktop_template")]
    pub desktop_template: Option<PathBuf>,
    /// Define the section in Debian Control file. See : <https://www.debian.org/doc/debian-policy/ch-archive.html#s-subsections>
    pub section: Option<String>,
    /// Change the priority of the Debian Package. By default, it is set to `optional`.
    /// Recognized Priorities as of now are :  `required`, `important`, `standard`, `optional`, `extra`
    pub priority: Option<String>,
    /// List of custom files to add to the deb package.
    /// Maps a dir/file to a dir/file inside the debian package.
    pub files: Option<HashMap<String, String>>,
}

impl DebianConfig {
    /// Creates a new [`DebianConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the list of Debian dependencies directly using an iterator of strings.
    pub fn depends<I, S>(mut self, depends: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.depends
            .replace(Dependencies::List(depends.into_iter().map(Into::into).collect()));
        self
    }

    /// Set the list of Debian dependencies indirectly via a path to a file,
    /// which must contain one dependency (a package name) per line.
    pub fn depends_path<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>
    {
        self.depends
            .replace(Dependencies::Path(path.into()));
        self
    }

    /// Set the path to a custom desktop file Handlebars template.
    ///
    /// Available variables: `categories`, `comment` (optional), `exec`, `icon` and `name`.
    ///
    /// Default file contents:
    /// ```text
    /// [Desktop Entry]
    /// Categories={{categories}}
    /// {{#if comment}}
    /// Comment={{comment}}
    /// {{/if}}
    /// Exec={{exec}}
    /// Icon={{icon}}
    /// Name={{name}}
    /// Terminal=false
    /// Type=Application
    /// {{#if mime_type}}
    /// MimeType={{mime_type}}
    /// {{/if}}
    /// ```
    pub fn desktop_template<P: Into<PathBuf>>(mut self, desktop_template: P) -> Self {
        self.desktop_template.replace(desktop_template.into());
        self
    }

    /// Define the section in Debian Control file. See : <https://www.debian.org/doc/debian-policy/ch-archive.html#s-subsections>
    pub fn section<S: Into<String>>(mut self, section: S) -> Self {
        self.section.replace(section.into());
        self
    }

    /// Change the priority of the Debian Package. By default, it is set to `optional`.
    /// Recognized Priorities as of now are :  `required`, `important`, `standard`, `optional`, `extra`
    pub fn priority<S: Into<String>>(mut self, priority: S) -> Self {
        self.priority.replace(priority.into());
        self
    }

    /// Set the list of custom files to add to the deb package.
    /// Maps a dir/file to a dir/file inside the debian package.
    pub fn files<I, S, T>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
        S: Into<String>,
        T: Into<String>,
    {
        self.files.replace(
            files
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }
}


/// A list of dependencies specified as either a list of Strings
/// or as a path to a file that lists the dependencies, one per line.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum Dependencies {
    /// The list of dependencies provided directly as a vector of Strings.
    List(Vec<String>),
    /// A path to the file containing the list of dependences, formatted as one per line:
    /// ```text
    /// libc6
    /// libxcursor1
    /// libdbus-1-3
    /// libasyncns0
    /// ...
    /// ```
    Path(PathBuf),
}
impl Dependencies {
    /// Returns the dependencies as a list of Strings.
    pub fn to_list(&self) -> crate::Result<Vec<String>> {
        match self {
            Self::List(v) => Ok(v.clone()),
            Self::Path(path) => {
                let trimmed_lines = std::fs::read_to_string(path)?
                    .lines()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            Some(trimmed.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(trimmed_lines)
            }
        }
    }
}

/// The Linux AppImage configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct AppImageConfig {
    /// List of libs that exist in `/usr/lib*` to be include in the final AppImage.
    /// The libs will be searched for, using the command
    /// `find -L /usr/lib* -name <libname>`
    pub libs: Option<Vec<String>>,
    /// List of binary paths to include in the final AppImage.
    /// For example, if you want `xdg-open`, you'd specify `/usr/bin/xdg-open`
    pub bins: Option<Vec<String>>,
    /// List of custom files to add to the appimage package.
    /// Maps a dir/file to a dir/file inside the appimage package.
    pub files: Option<HashMap<String, String>>,
    /// A map of [`linuxdeploy`](https://github.com/linuxdeploy/linuxdeploy)
    /// plugin name and its URL to be downloaded and executed while packaing the appimage.
    /// For example, if you want to use the
    /// [`gtk`](https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh) plugin,
    /// you'd specify `gtk` as the key and its url as the value.
    #[serde(alias = "linuxdeploy-plugins", alias = "linuxdeploy_plugins")]
    pub linuxdeploy_plugins: Option<HashMap<String, String>>,
    /// List of globs of libraries to exclude from the final AppImage.
    /// For example, to exclude libnss3.so, you'd specify `libnss3*`
    #[serde(alias = "excluded-libraries", alias = "excluded_libraries")]
    pub excluded_libs: Option<Vec<String>>,
}

impl AppImageConfig {
    /// Creates a new [`DebianConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the list of libs that exist in `/usr/lib*` to be include in the final AppImage.
    /// The libs will be searched for using, the command
    /// `find -L /usr/lib* -name <libname>`
    pub fn libs<I, S>(mut self, libs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.libs
            .replace(libs.into_iter().map(Into::into).collect());
        self
    }

    /// Set the list of binary paths to include in the final AppImage.
    /// For example, if you want `xdg-open`, you'd specify `/usr/bin/xdg-open`
    pub fn bins<I, S>(mut self, bins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.bins
            .replace(bins.into_iter().map(Into::into).collect());
        self
    }

    /// Set the list of custom files to add to the appimage package.
    /// Maps a dir/file to a dir/file inside the appimage package.
    pub fn files<I, S, T>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
        S: Into<String>,
        T: Into<String>,
    {
        self.files.replace(
            files
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Set the map of [`linuxdeploy`](https://github.com/linuxdeploy/linuxdeploy)
    /// plugin name and its URL to be downloaded and executed while packaing the appimage.
    /// For example, if you want to use the
    /// [`gtk`](https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh) plugin,
    /// you'd specify `gtk` as the key and its url as the value.
    pub fn linuxdeploy_plugins<I, S, T>(mut self, linuxdeploy_plugins: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
        S: Into<String>,
        T: Into<String>,
    {
        self.linuxdeploy_plugins.replace(
            linuxdeploy_plugins
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }
}

/// The Linux pacman configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct PacmanConfig {
    /// List of custom files to add to the pacman package.
    /// Maps a dir/file to a dir/file inside the pacman package.
    pub files: Option<HashMap<String, String>>,
    /// List of softwares that must be installed for the app to build and run.
    ///
    /// See : <https://wiki.archlinux.org/title/PKGBUILD#depends>
    pub depends: Option<Dependencies>,
    /// Additional packages that are provided by this app.
    ///
    /// See : <https://wiki.archlinux.org/title/PKGBUILD#provides>
    pub provides: Option<Vec<String>>,
    /// Packages that conflict or cause problems with the app.
    /// All these packages and packages providing this item will need to be removed
    ///
    /// See : <https://wiki.archlinux.org/title/PKGBUILD#conflicts>
    pub conflicts: Option<Vec<String>>,
    /// Only use if this app replaces some obsolete packages.
    /// For example, if you rename any package.
    ///
    /// See : <https://wiki.archlinux.org/title/PKGBUILD#replaces>
    pub replaces: Option<Vec<String>>,
    /// Source of the package to be stored at PKGBUILD.
    /// PKGBUILD is a bash script, so version can be referred as ${pkgver}
    pub source: Option<Vec<String>>,
}

impl PacmanConfig {
    /// Creates a new [`PacmanConfig`].
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the list of custom files to add to the pacman package.
    /// Maps a dir/file to a dir/file inside the pacman package.
    pub fn files<I, S, T>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
        S: Into<String>,
        T: Into<String>,
    {
        self.files.replace(
            files
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Set the list of pacman dependencies directly using an iterator of strings.
    pub fn depends<I, S>(mut self, depends: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.depends
            .replace(Dependencies::List(depends.into_iter().map(Into::into).collect()));
        self
    }

    /// Set the list of pacman dependencies indirectly via a path to a file,
    /// which must contain one dependency (a package name) per line.
    pub fn depends_path<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>
    {
        self.depends
            .replace(Dependencies::Path(path.into()));
        self
    }

    /// Set the list of additional packages that are provided by this app.
    pub fn provides<I, S>(mut self, provides: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.provides
            .replace(provides.into_iter().map(Into::into).collect());
        self
    }
    /// Set the list of packages that conflict with the app.
    pub fn conflicts<I, S>(mut self, conflicts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.conflicts
            .replace(conflicts.into_iter().map(Into::into).collect());
        self
    }
    /// Set the list of obsolete packages that are replaced by this package.
    pub fn replaces<I, S>(mut self, replaces: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.replaces
            .replace(replaces.into_iter().map(Into::into).collect());
        self
    }
    /// Set the list of sources where the package will be stored.
    pub fn source<I, S>(mut self, source: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.source
            .replace(source.into_iter().map(Into::into).collect());
        self
    }
}

/// Position coordinates struct.
#[derive(Default, Copy, Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Position {
    /// X coordinate.
    pub x: u32,
    /// Y coordinate.
    pub y: u32,
}

/// Size struct.
#[derive(Default, Copy, Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Size {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// The Apple Disk Image (.dmg) configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct DmgConfig {
    /// Image to use as the background in dmg file. Accepted formats: `png`/`jpg`/`gif`.
    pub background: Option<PathBuf>,
    /// Position of volume window on screen.
    pub window_position: Option<Position>,
    /// Size of volume window.
    #[serde(alias = "window-size", alias = "window_size")]
    pub window_size: Option<Size>,
    /// Position of application file on window.
    #[serde(alias = "app-position", alias = "app_position")]
    pub app_position: Option<Position>,
    /// Position of application folder on window.
    #[serde(
        alias = "application-folder-position",
        alias = "application_folder_position"
    )]
    pub app_folder_position: Option<Position>,
}

impl DmgConfig {
    /// Creates a new [`DmgConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an image to use as the background in dmg file. Accepted formats: `png`/`jpg`/`gif`.
    pub fn background<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.background.replace(path.into());
        self
    }

    /// Set the poosition of volume window on screen.
    pub fn window_position(mut self, position: Position) -> Self {
        self.window_position.replace(position);
        self
    }

    /// Set the size of volume window.
    pub fn window_size(mut self, size: Size) -> Self {
        self.window_size.replace(size);
        self
    }

    /// Set the poosition of app file on window.
    pub fn app_position(mut self, position: Position) -> Self {
        self.app_position.replace(position);
        self
    }

    /// Set the position of application folder on window.
    pub fn app_folder_position(mut self, position: Position) -> Self {
        self.app_folder_position.replace(position);
        self
    }
}

/// Notarization authentication credentials.
#[derive(Clone, Debug)]
pub enum MacOsNotarizationCredentials {
    /// Apple ID authentication.
    AppleId {
        /// Apple ID.
        apple_id: OsString,
        /// Password.
        password: OsString,
        /// Team ID.
        team_id: OsString,
    },
    /// App Store Connect API key.
    ApiKey {
        /// API key issuer.
        issuer: OsString,
        /// API key ID.
        key_id: OsString,
        /// Path to the API key file.
        key_path: PathBuf,
    },
}

/// The macOS configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct MacOsConfig {
    /// MacOS frameworks that need to be packaged with the app.
    ///
    /// Each string can either be the name of a framework (without the `.framework` extension, e.g. `"SDL2"`),
    /// in which case we will search for that framework in the standard install locations (`~/Library/Frameworks/`, `/Library/Frameworks/`, and `/Network/Library/Frameworks/`),
    /// or a path to a specific framework bundle (e.g. `./data/frameworks/SDL2.framework`).  Note that this setting just makes cargo-packager copy the specified frameworks into the OS X app bundle
    /// (under `Foobar.app/Contents/Frameworks/`); you are still responsible for:
    ///
    /// - arranging for the compiled binary to link against those frameworks (e.g. by emitting lines like `cargo:rustc-link-lib=framework=SDL2` from your `build.rs` script)
    ///
    /// - embedding the correct rpath in your binary (e.g. by running `install_name_tool -add_rpath "@executable_path/../Frameworks" path/to/binary` after compiling)
    pub frameworks: Option<Vec<String>>,
    /// A version string indicating the minimum MacOS version that the packaged app supports (e.g. `"10.11"`).
    /// If you are using this config field, you may also want have your `build.rs` script emit `cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.11`.
    #[serde(alias = "minimum-system-version", alias = "minimum_system_version")]
    pub minimum_system_version: Option<String>,
    /// The exception domain to use on the macOS .app package.
    ///
    /// This allows communication to the outside world e.g. a web server you're shipping.
    #[serde(alias = "exception-domain", alias = "exception_domain")]
    pub exception_domain: Option<String>,
    /// Code signing identity.
    #[serde(alias = "signing-identity", alias = "signing_identity")]
    pub signing_identity: Option<String>,
    /// Codesign certificate (base64 encoded of the p12 file).
    #[serde(skip)]
    pub signing_certificate: Option<OsString>,
    /// Password of the codesign certificate.
    #[serde(skip)]
    pub signing_certificate_password: Option<OsString>,
    /// Notarization authentication credentials.
    #[serde(skip)]
    pub notarization_credentials: Option<MacOsNotarizationCredentials>,
    /// Provider short name for notarization.
    #[serde(alias = "provider-short-name", alias = "provider_short_name")]
    pub provider_short_name: Option<String>,
    /// Path to the entitlements.plist file.
    pub entitlements: Option<String>,
    /// Path to the Info.plist file for the package.
    #[serde(alias = "info-plist-path", alias = "info_plist_path")]
    pub info_plist_path: Option<PathBuf>,
}

impl MacOsConfig {
    /// Creates a new [`MacOsConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// MacOS frameworks that need to be packaged with the app.
    ///
    /// Each string can either be the name of a framework (without the `.framework` extension, e.g. `"SDL2"`),
    /// in which case we will search for that framework in the standard install locations (`~/Library/Frameworks/`, `/Library/Frameworks/`, and `/Network/Library/Frameworks/`),
    /// or a path to a specific framework bundle (e.g. `./data/frameworks/SDL2.framework`).  Note that this setting just makes cargo-packager copy the specified frameworks into the OS X app bundle
    /// (under `Foobar.app/Contents/Frameworks/`); you are still responsible for:
    ///
    /// - arranging for the compiled binary to link against those frameworks (e.g. by emitting lines like `cargo:rustc-link-lib=framework=SDL2` from your `build.rs` script)
    ///
    /// - embedding the correct rpath in your binary (e.g. by running `install_name_tool -add_rpath "@executable_path/../Frameworks" path/to/binary` after compiling)
    pub fn frameworks<I, S>(mut self, frameworks: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.frameworks
            .replace(frameworks.into_iter().map(Into::into).collect());
        self
    }

    /// A version string indicating the minimum MacOS version that the packaged app supports (e.g. `"10.11"`).
    /// If you are using this config field, you may also want have your `build.rs` script emit `cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.11`.
    pub fn minimum_system_version<S: Into<String>>(mut self, minimum_system_version: S) -> Self {
        self.minimum_system_version
            .replace(minimum_system_version.into());
        self
    }

    /// The exception domain to use on the macOS .app package.
    ///
    /// This allows communication to the outside world e.g. a web server you're shipping.
    pub fn exception_domain<S: Into<String>>(mut self, exception_domain: S) -> Self {
        self.exception_domain.replace(exception_domain.into());
        self
    }

    /// Code signing identity.
    pub fn signing_identity<S: Into<String>>(mut self, signing_identity: S) -> Self {
        self.signing_identity.replace(signing_identity.into());
        self
    }

    /// Provider short name for notarization.
    pub fn provider_short_name<S: Into<String>>(mut self, provider_short_name: S) -> Self {
        self.provider_short_name.replace(provider_short_name.into());
        self
    }

    /// Path to the entitlements.plist file.
    pub fn entitlements<S: Into<String>>(mut self, entitlements: S) -> Self {
        self.entitlements.replace(entitlements.into());
        self
    }

    /// Path to the Info.plist file for the package.
    pub fn info_plist_path<S: Into<PathBuf>>(mut self, info_plist_path: S) -> Self {
        self.info_plist_path.replace(info_plist_path.into());
        self
    }
}

/// A wix language.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum WixLanguage {
    /// Built-in wix language identifier.
    Identifier(String),
    /// Custom wix language.
    Custom {
        /// Idenitifier of this language, for example `en-US`
        identifier: String,
        /// The path to a locale (`.wxl`) file. See <https://wixtoolset.org/documentation/manual/v3/howtos/ui_and_localization/build_a_localized_version.html>.
        path: Option<PathBuf>,
    },
}

impl Default for WixLanguage {
    fn default() -> Self {
        Self::Identifier("en-US".into())
    }
}

/// The wix format configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct WixConfig {
    /// The app languages to build. See <https://docs.microsoft.com/en-us/windows/win32/msi/localizing-the-error-and-actiontext-tables>.
    pub languages: Option<Vec<WixLanguage>>,
    /// By default, the packager uses an internal template.
    /// This option allows you to define your own wix file.
    pub template: Option<PathBuf>,
    /// List of merge modules to include in your installer.
    /// For example, if you want to include [C++ Redis merge modules]
    ///
    /// [C++ Redis merge modules]: https://wixtoolset.org/docs/v3/howtos/redistributables_and_install_checks/install_vcredist/
    #[serde(alias = "merge-modules", alias = "merge_modules")]
    pub merge_modules: Option<Vec<PathBuf>>,
    /// A list of paths to .wxs files with WiX fragments to use.
    #[serde(alias = "fragment-paths", alias = "fragment_paths")]
    pub fragment_paths: Option<Vec<PathBuf>>,
    /// List of WiX fragments as strings. This is similar to `config.wix.fragments_paths` but
    /// is a string so you can define it inline in your config.
    ///
    /// ```text
    /// <?xml version="1.0" encoding="utf-8"?>
    /// <Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    /// <Fragment>
    ///     <CustomAction Id="OpenNotepad" Directory="INSTALLDIR" Execute="immediate" ExeCommand="cmd.exe /c notepad.exe" Return="check" />
    ///     <InstallExecuteSequence>
    ///         <Custom Action="OpenNotepad" After="InstallInitialize" />
    ///     </InstallExecuteSequence>
    /// </Fragment>
    /// </Wix>
    /// ```
    pub fragments: Option<Vec<String>>,
    /// The ComponentGroup element ids you want to reference from the fragments.
    #[serde(alias = "component-group-refs", alias = "component_group_refs")]
    pub component_group_refs: Option<Vec<String>>,
    /// The Component element ids you want to reference from the fragments.
    #[serde(alias = "component-refs", alias = "component_refs")]
    pub component_refs: Option<Vec<String>>,
    /// The CustomAction element ids you want to reference from the fragments.
    #[serde(alias = "custom-action-refs", alias = "custom_action_refs")]
    pub custom_action_refs: Option<Vec<String>>,
    /// The FeatureGroup element ids you want to reference from the fragments.
    #[serde(alias = "feature-group-refs", alias = "feature_group_refs")]
    pub feature_group_refs: Option<Vec<String>>,
    /// The Feature element ids you want to reference from the fragments.
    #[serde(alias = "feature-refs", alias = "feature_refs")]
    pub feature_refs: Option<Vec<String>>,
    /// The Merge element ids you want to reference from the fragments.
    #[serde(alias = "merge-refs", alias = "merge_refs")]
    pub merge_refs: Option<Vec<String>>,
    /// Path to a bitmap file to use as the installation user interface banner.
    /// This bitmap will appear at the top of all but the first page of the installer.
    ///
    /// The required dimensions are 493px × 58px.
    #[serde(alias = "banner-path", alias = "banner_path")]
    pub banner_path: Option<PathBuf>,
    /// Path to a bitmap file to use on the installation user interface dialogs.
    /// It is used on the welcome and completion dialogs.
    /// The required dimensions are 493px × 312px.
    #[serde(alias = "dialog-image-path", alias = "dialog_image_path")]
    pub dialog_image_path: Option<PathBuf>,
    /// Enables FIPS compliant algorithms.
    #[serde(default, alias = "fips-compliant", alias = "fips_compliant")]
    pub fips_compliant: bool,
}

impl WixConfig {
    /// Creates a new [`WixConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the app languages to build. See <https://docs.microsoft.com/en-us/windows/win32/msi/localizing-the-error-and-actiontext-tables>.
    pub fn languages<I: IntoIterator<Item = WixLanguage>>(mut self, languages: I) -> Self {
        self.languages.replace(languages.into_iter().collect());
        self
    }

    /// By default, the packager uses an internal template.
    /// This option allows you to define your own wix file.
    pub fn template<P: Into<PathBuf>>(mut self, template: P) -> Self {
        self.template.replace(template.into());
        self
    }

    /// Set a list of merge modules to include in your installer.
    /// For example, if you want to include [C++ Redis merge modules]
    ///
    /// [C++ Redis merge modules]: https://wixtoolset.org/docs/v3/howtos/redistributables_and_install_checks/install_vcredist/
    pub fn merge_modules<I, P>(mut self, merge_modules: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.merge_modules
            .replace(merge_modules.into_iter().map(Into::into).collect());
        self
    }

    /// Set a list of paths to .wxs files with WiX fragments to use.
    pub fn fragment_paths<I, S>(mut self, fragment_paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<PathBuf>,
    {
        self.fragment_paths
            .replace(fragment_paths.into_iter().map(Into::into).collect());
        self
    }

    /// Set a list of WiX fragments as strings. This is similar to [`WixConfig::fragment_paths`] but
    /// is a string so you can define it inline in your config.
    ///
    /// ```text
    /// <?xml version="1.0" encoding="utf-8"?>
    /// <Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    /// <Fragment>
    ///     <CustomAction Id="OpenNotepad" Directory="INSTALLDIR" Execute="immediate" ExeCommand="cmd.exe /c notepad.exe" Return="check" />
    ///     <InstallExecuteSequence>
    ///         <Custom Action="OpenNotepad" After="InstallInitialize" />
    ///     </InstallExecuteSequence>
    /// </Fragment>
    /// </Wix>
    /// ```
    pub fn fragments<I, S>(mut self, fragments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.fragments
            .replace(fragments.into_iter().map(Into::into).collect());
        self
    }

    /// Set the ComponentGroup element ids you want to reference from the fragments.
    pub fn component_group_refs<I, S>(mut self, component_group_refs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.component_group_refs
            .replace(component_group_refs.into_iter().map(Into::into).collect());
        self
    }

    /// Set the Component element ids you want to reference from the fragments.
    pub fn component_refs<I, S>(mut self, component_refs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.component_refs
            .replace(component_refs.into_iter().map(Into::into).collect());
        self
    }

    /// Set the CustomAction element ids you want to reference from the fragments.
    pub fn custom_action_refs<I, S>(mut self, custom_action_refs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.custom_action_refs
            .replace(custom_action_refs.into_iter().map(Into::into).collect());
        self
    }

    /// Set he FeatureGroup element ids you want to reference from the fragments.
    pub fn feature_group_refs<I, S>(mut self, feature_group_refs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.feature_group_refs
            .replace(feature_group_refs.into_iter().map(Into::into).collect());
        self
    }

    /// Set the Feature element ids you want to reference from the fragments.
    pub fn feature_refs<I, S>(mut self, feature_refs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.feature_refs
            .replace(feature_refs.into_iter().map(Into::into).collect());
        self
    }

    /// Set he Merge element ids you want to reference from the fragments.
    pub fn merge_refs<I, S>(mut self, merge_refs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.merge_refs
            .replace(merge_refs.into_iter().map(Into::into).collect());
        self
    }

    /// Set the path to a bitmap file to use as the installation user interface banner.
    /// This bitmap will appear at the top of all but the first page of the installer.
    ///
    /// The required dimensions are 493px × 58px.
    pub fn banner_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.banner_path.replace(path.into());
        self
    }

    /// Set the path to a bitmap file to use on the installation user interface dialogs.
    /// It is used on the welcome and completion dialogs.
    /// The required dimensions are 493px × 312px.
    pub fn dialog_image_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.dialog_image_path.replace(path.into());
        self
    }

    /// Set whether to enable or disable FIPS compliant algorithms.
    pub fn fips_compliant(mut self, fips_compliant: bool) -> Self {
        self.fips_compliant = fips_compliant;
        self
    }
}

/// Install Modes for the NSIS installer.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub enum NSISInstallerMode {
    /// Default mode for the installer.
    ///
    /// Install the app by default in a directory that doesn't require Administrator access.
    ///
    /// Installer metadata will be saved under the `HKCU` registry path.
    CurrentUser,
    /// Install the app by default in the `Program Files` folder directory requires Administrator
    /// access for the installation.
    ///
    /// Installer metadata will be saved under the `HKLM` registry path.
    PerMachine,
    /// Combines both modes and allows the user to choose at install time
    /// whether to install for the current user or per machine. Note that this mode
    /// will require Administrator access even if the user wants to install it for the current user only.
    ///
    /// Installer metadata will be saved under the `HKLM` or `HKCU` registry path based on the user's choice.
    Both,
}

impl Default for NSISInstallerMode {
    fn default() -> Self {
        Self::CurrentUser
    }
}

/// Compression algorithms used in the NSIS installer.
///
/// See <https://nsis.sourceforge.io/Reference/SetCompressor>
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum NsisCompression {
    /// ZLIB uses the deflate algorithm, it is a quick and simple method. With the default compression level it uses about 300 KB of memory.
    Zlib,
    /// BZIP2 usually gives better compression ratios than ZLIB, but it is a bit slower and uses more memory. With the default compression level it uses about 4 MB of memory.
    Bzip2,
    /// LZMA (default) is a new compression method that gives very good compression ratios. The decompression speed is high (10-20 MB/s on a 2 GHz CPU), the compression speed is lower. The memory size that will be used for decompression is the dictionary size plus a few KBs, the default is 8 MB.
    Lzma,
    /// Disable compression.
    Off,
}

/// The NSIS format configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct NsisConfig {
    /// Set the compression algorithm used to compress files in the installer.
    ///
    /// See <https://nsis.sourceforge.io/Reference/SetCompressor>
    pub compression: Option<NsisCompression>,
    /// A custom `.nsi` template to use.
    ///
    /// See the default template here
    /// <https://github.com/crabnebula-dev/cargo-packager/blob/main/crates/packager/src/nsis/installer.nsi>
    pub template: Option<PathBuf>,
    /// Logic of an NSIS section that will be ran before the install section.
    ///
    /// See the available libraries, dlls and global variables here
    /// <https://github.com/crabnebula-dev/cargo-packager/blob/main/crates/packager/src/nsis/installer.nsi>
    ///
    /// ### Example
    /// ```toml
    /// [package.metadata.packager.nsis]
    /// preinstall-section = """
    ///     ; Setup custom messages
    ///     LangString webview2AbortError ${LANG_ENGLISH} "Failed to install WebView2! The app can't run without it. Try restarting the installer."
    ///     LangString webview2DownloadError ${LANG_ARABIC} "خطأ: فشل تنزيل WebView2 - $0"
    ///
    ///     Section PreInstall
    ///      ; <section logic here>
    ///     SectionEnd
    ///
    ///     Section AnotherPreInstall
    ///      ; <section logic here>
    ///     SectionEnd
    /// """
    /// ```
    #[serde(alias = "preinstall-section", alias = "preinstall_section")]
    pub preinstall_section: Option<String>,
    /// The path to a bitmap file to display on the header of installers pages.
    ///
    /// The recommended dimensions are 150px x 57px.
    #[serde(alias = "header-image", alias = "header_image")]
    pub header_image: Option<PathBuf>,
    /// The path to a bitmap file for the Welcome page and the Finish page.
    ///
    /// The recommended dimensions are 164px x 314px.
    #[serde(alias = "sidebar-image", alias = "sidebar_image")]
    pub sidebar_image: Option<PathBuf>,
    /// The path to an icon file used as the installer icon.
    #[serde(alias = "installer-icon", alias = "installer_icon")]
    pub installer_icon: Option<PathBuf>,
    /// Whether the installation will be for all users or just the current user.
    #[serde(default, alias = "installer-mode", alias = "installer_mode")]
    pub install_mode: NSISInstallerMode,
    /// A list of installer languages.
    /// By default the OS language is used. If the OS language is not in the list of languages, the first language will be used.
    /// To allow the user to select the language, set `display_language_selector` to `true`.
    ///
    /// See <https://github.com/kichik/nsis/tree/9465c08046f00ccb6eda985abbdbf52c275c6c4d/Contrib/Language%20files> for the complete list of languages.
    pub languages: Option<Vec<String>>,
    /// An key-value pair where the key is the language and the
    /// value is the path to a custom `.nsi` file that holds the translated text for cargo-packager's custom messages.
    ///
    /// See <https://github.com/crabnebula-dev/cargo-packager/blob/main/crates/packager/src/nsis/languages/English.nsh> for an example `.nsi` file.
    ///
    /// **Note**: the key must be a valid NSIS language and it must be added to [`NsisConfig`]languages array,
    #[serde(alias = "custom-language-file", alias = "custom_language_file")]
    pub custom_language_files: Option<HashMap<String, PathBuf>>,
    /// Whether to display a language selector dialog before the installer and uninstaller windows are rendered or not.
    /// By default the OS language is selected, with a fallback to the first language in the `languages` array.
    #[serde(
        default,
        alias = "display-language-selector",
        alias = "display_language_selector"
    )]
    pub display_language_selector: bool,
    /// List of paths where your app stores data.
    /// This options tells the uninstaller to provide the user with an option
    /// (disabled by default) whether they want to rmeove your app data or keep it.
    ///
    /// The path should use a constant from <https://nsis.sourceforge.io/Docs/Chapter4.html#varconstant>
    /// in addition to `$IDENTIFIER`, `$PUBLISHER` and `$PRODUCTNAME`, for example, if you store your
    /// app data in `C:\\Users\\<user>\\AppData\\Local\\<your-company-name>\\<your-product-name>`
    /// you'd need to specify
    /// ```toml
    /// [package.metadata.packager.nsis]
    /// appdata-paths = ["$LOCALAPPDATA/$PUBLISHER/$PRODUCTNAME"]
    /// ```
    #[serde(default, alias = "appdata-paths", alias = "appdata_paths")]
    pub appdata_paths: Option<Vec<String>>,
}

impl NsisConfig {
    /// Creates a new [`NsisConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the compression algorithm used to compress files in the installer.
    ///
    /// See <https://nsis.sourceforge.io/Reference/SetCompressor>
    pub fn compression(mut self, compression: NsisCompression) -> Self {
        self.compression.replace(compression);
        self
    }

    /// Set a custom `.nsi` template to use.
    ///
    /// See the default template here
    /// <https://github.com/crabnebula-dev/cargo-packager/blob/main/crates/packager/src/nsis/installer.nsi>
    pub fn template<P: Into<PathBuf>>(mut self, template: P) -> Self {
        self.template.replace(template.into());
        self
    }

    /// Set the logic of an NSIS section that will be ran before the install section.
    ///
    /// See the available libraries, dlls and global variables here
    /// <https://github.com/crabnebula-dev/cargo-packager/blob/main/crates/packager/src/nsis/installer.nsi>
    ///
    /// ### Example
    /// ```toml
    /// [package.metadata.packager.nsis]
    /// preinstall-section = """
    ///     ; Setup custom messages
    ///     LangString webview2AbortError ${LANG_ENGLISH} "Failed to install WebView2! The app can't run without it. Try restarting the installer."
    ///     LangString webview2DownloadError ${LANG_ARABIC} "خطأ: فشل تنزيل WebView2 - $0"
    ///
    ///     Section PreInstall
    ///      ; <section logic here>
    ///     SectionEnd
    ///
    ///     Section AnotherPreInstall
    ///      ; <section logic here>
    ///     SectionEnd
    /// """
    /// ```
    pub fn preinstall_section<S: Into<String>>(mut self, preinstall_section: S) -> Self {
        self.preinstall_section.replace(preinstall_section.into());
        self
    }

    /// Set the path to a bitmap file to display on the header of installers pages.
    ///
    /// The recommended dimensions are 150px x 57px.
    pub fn header_image<P: Into<PathBuf>>(mut self, header_image: P) -> Self {
        self.header_image.replace(header_image.into());
        self
    }

    /// Set the path to a bitmap file for the Welcome page and the Finish page.
    ///
    /// The recommended dimensions are 164px x 314px.
    pub fn sidebar_image<P: Into<PathBuf>>(mut self, sidebar_image: P) -> Self {
        self.sidebar_image.replace(sidebar_image.into());
        self
    }

    /// Set the path to an icon file used as the installer icon.
    pub fn installer_icon<P: Into<PathBuf>>(mut self, installer_icon: P) -> Self {
        self.installer_icon.replace(installer_icon.into());
        self
    }

    /// Set whether the installation will be for all users or just the current user.
    pub fn install_mode(mut self, install_mode: NSISInstallerMode) -> Self {
        self.install_mode = install_mode;
        self
    }

    /// Set a list of installer languages.
    /// By default the OS language is used. If the OS language is not in the list of languages, the first language will be used.
    /// To allow the user to select the language, set `display_language_selector` to `true`.
    ///
    /// See <https://github.com/kichik/nsis/tree/9465c08046f00ccb6eda985abbdbf52c275c6c4d/Contrib/Language%20files> for the complete list of languages.
    pub fn languages<I, S>(mut self, languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.languages
            .replace(languages.into_iter().map(Into::into).collect());
        self
    }

    /// Set a map of key-value pair where the key is the language and the
    /// value is the path to a custom `.nsi` file that holds the translated text for cargo-packager's custom messages.
    ///
    /// See <https://github.com/crabnebula-dev/cargo-packager/blob/main/crates/packager/src/nsis/languages/English.nsh> for an example `.nsi` file.
    ///
    /// **Note**: the key must be a valid NSIS language and it must be added to [`NsisConfig`]languages array,
    pub fn custom_language_files<I, S, P>(mut self, custom_language_files: I) -> Self
    where
        I: IntoIterator<Item = (S, P)>,
        S: Into<String>,
        P: Into<PathBuf>,
    {
        self.custom_language_files.replace(
            custom_language_files
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Set wether to display a language selector dialog before the installer and uninstaller windows are rendered or not.
    /// By default the OS language is selected, with a fallback to the first language in the `languages` array.
    pub fn display_language_selector(mut self, display: bool) -> Self {
        self.display_language_selector = display;
        self
    }

    /// Set a list of paths where your app stores data.
    /// This options tells the uninstaller to provide the user with an option
    /// (disabled by default) whether they want to rmeove your app data or keep it.
    ///
    /// The path should use a constant from <https://nsis.sourceforge.io/Docs/Chapter4.html#varconstant>
    /// in addition to `$IDENTIFIER`, `$PUBLISHER` and `$PRODUCTNAME`, for example, if you store your
    /// app data in `C:\\Users\\<user>\\AppData\\Local\\<your-company-name>\\<your-product-name>`
    /// you'd need to specify
    /// ```toml
    /// [package.metadata.packager.nsis]
    /// appdata-paths = ["$LOCALAPPDATA/$PUBLISHER/$PRODUCTNAME"]
    /// ```
    pub fn appdata_paths<I, S>(mut self, appdata_paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.appdata_paths
            .replace(appdata_paths.into_iter().map(Into::into).collect());
        self
    }
}

/// The Windows configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct WindowsConfig {
    /// The file digest algorithm to use for creating file signatures. Required for code signing. SHA-256 is recommended.
    #[serde(alias = "digest-algorithim", alias = "digest_algorithim")]
    pub digest_algorithm: Option<String>,
    /// The SHA1 hash of the signing certificate.
    #[serde(alias = "certificate-thumbprint", alias = "certificate_thumbprint")]
    pub certificate_thumbprint: Option<String>,
    /// Whether to use Time-Stamp Protocol (TSP, a.k.a. RFC 3161) for the timestamp server. Your code signing provider may
    /// use a TSP timestamp server, like e.g. SSL.com does. If so, enable TSP by setting to true.
    #[serde(default)]
    pub tsp: bool,
    /// Server to use during timestamping.
    #[serde(alias = "timestamp-url", alias = "timestamp_url")]
    pub timestamp_url: Option<String>,
    /// Whether to validate a second app installation, blocking the user from installing an older version if set to `false`.
    ///
    /// For instance, if `1.2.1` is installed, the user won't be able to install app version `1.2.0` or `1.1.5`.
    ///
    /// The default value of this flag is `true`.
    #[serde(
        default = "default_true",
        alias = "allow-downgrades",
        alias = "allow_downgrades"
    )]
    pub allow_downgrades: bool,

    /// Specify a custom command to sign the binaries.
    /// This command needs to have a `%1` in it which is just a placeholder for the binary path,
    /// which we will detect and replace before calling the command.
    ///
    /// By Default we use `signtool.exe` which can be found only on Windows so
    /// if you are on another platform and want to cross-compile and sign you will
    /// need to use another tool like `osslsigncode`.
    #[serde(alias = "sign-command", alias = "sign_command")]
    pub sign_command: Option<String>,
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            digest_algorithm: None,
            certificate_thumbprint: None,
            timestamp_url: None,
            tsp: false,
            allow_downgrades: true,
            sign_command: None,
        }
    }
}

impl WindowsConfig {
    /// Creates a new [`WindowsConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the file digest algorithm to use for creating file signatures. Required for code signing. SHA-256 is recommended.
    pub fn digest_algorithm<S: Into<String>>(mut self, digest_algorithm: S) -> Self {
        self.digest_algorithm.replace(digest_algorithm.into());
        self
    }

    /// Set the SHA1 hash of the signing certificate.
    pub fn certificate_thumbprint<S: Into<String>>(mut self, certificate_thumbprint: S) -> Self {
        self.certificate_thumbprint
            .replace(certificate_thumbprint.into());
        self
    }

    /// Set whether to use Time-Stamp Protocol (TSP, a.k.a. RFC 3161) for the timestamp server. Your code signing provider may
    /// use a TSP timestamp server, like e.g. SSL.com does. If so, enable TSP by setting to true.
    pub fn tsp(mut self, tsp: bool) -> Self {
        self.tsp = tsp;
        self
    }

    /// Set server url to use during timestamping.
    pub fn timestamp_url<S: Into<String>>(mut self, timestamp_url: S) -> Self {
        self.timestamp_url.replace(timestamp_url.into());
        self
    }

    /// Set whether to validate a second app installation, blocking the user from installing an older version if set to `false`.
    ///
    /// For instance, if `1.2.1` is installed, the user won't be able to install app version `1.2.0` or `1.1.5`.
    ///
    /// The default value of this flag is `true`.
    pub fn allow_downgrades(mut self, allow: bool) -> Self {
        self.allow_downgrades = allow;
        self
    }
}

/// An enum representing the available verbosity levels of the logger.
#[derive(Deserialize, Serialize)]
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum LogLevel {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 1,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Error
    }
}

/// A binary to package within the final package.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct Binary {
    /// Path to the binary (without `.exe` on Windows).
    /// If it's relative, it will be resolved from [`Config::out_dir`].
    pub path: PathBuf,
    /// Whether this is the main binary or not
    #[serde(default)]
    pub main: bool,
}

impl Binary {
    /// Creates a new [`Binary`] from a path to the binary (without `.exe` on Windows).
    /// If it's relative, it will be resolved from [`Config::out_dir`].
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            path: path.into(),
            main: false,
        }
    }

    /// Set the path of the binary.
    pub fn path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.path = path.into();
        self
    }

    /// Set the binary as main binary.
    pub fn main(mut self, main: bool) -> Self {
        self.main = main;
        self
    }
}

/// A path to a resource (with optional glob pattern)
/// or an object of `src` and `target` paths.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum Resource {
    /// Supports glob patterns
    Single(String),
    /// An object descriping the src file or directory
    /// and its target location in the final package.
    Mapped {
        /// The src file or directory, supports glob patterns.
        src: String,
        /// A relative path from the root of the final package.
        ///
        /// If `src` is a glob, this will always be treated as a directory
        /// where all globbed files will be placed under.
        target: PathBuf,
    },
}

/// Describes a shell command to be executed when a CLI hook is triggered.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum HookCommand {
    /// Run the given script with the default options.
    Script(String),
    /// Run the given script with custom options.
    ScriptWithOptions {
        /// The script to execute.
        script: String,
        /// The working directory.
        dir: Option<String>,
    },
}

/// The packaging config.
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// The JSON schema for the config.
    ///
    /// Setting this field has no effect, this just exists so
    /// we can parse the JSON correctly when it has `$schema` field set.
    #[serde(rename = "$schema")]
    schema: Option<String>,
    /// The app name, this is just an identifier that could be used
    /// to filter which app to package using `--packages` cli arg when there is multiple apps in the
    /// workspace or in the same config.
    ///
    /// This field resembles, the `name` field in `Cargo.toml` or `package.json`
    ///
    /// If `unset`, the CLI will try to auto-detect it from `Cargo.toml` or
    /// `package.json` otherwise, it will keep it unset.
    pub(crate) name: Option<String>,
    /// Whether this config is enabled or not. Defaults to `true`.
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    /// The package's product name, for example "My Awesome App".
    #[serde(default, alias = "product-name", alias = "product_name")]
    pub product_name: String,
    /// The package's version.
    #[serde(default)]
    pub version: String,
    /// The binaries to package.
    #[serde(default)]
    pub binaries: Vec<Binary>,
    /// The application identifier in reverse domain name notation (e.g. `com.packager.example`).
    /// This string must be unique across applications since it is used in some system configurations.
    /// This string must contain only alphanumeric characters (A–Z, a–z, and 0–9), hyphens (-),
    /// and periods (.).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = r"^[a-zA-Z0-9-\.]*$")))]
    pub identifier: Option<String>,
    /// The command to run before starting to package an application.
    ///
    /// This runs only once.
    #[serde(alias = "before-packaging-command", alias = "before_packaging_command")]
    pub before_packaging_command: Option<HookCommand>,
    /// The command to run before packaging each format for an application.
    ///
    /// This will run multiple times depending on the formats specifed.
    #[serde(
        alias = "before-each-package-command",
        alias = "before_each_package_command"
    )]
    pub before_each_package_command: Option<HookCommand>,
    /// The logging level.
    #[serde(alias = "log-level", alias = "log_level")]
    pub log_level: Option<LogLevel>,
    /// The packaging formats to create, if not present, [`PackageFormat::platform_default`] is used.
    pub formats: Option<Vec<PackageFormat>>,
    /// The directory where the generated packages will be placed.
    ///
    /// If [`Config::binaries_dir`] is not set, this is also where the [`Config::binaries`] exist.
    #[serde(default, alias = "out-dir", alias = "out_dir")]
    pub out_dir: PathBuf,
    /// The directory where the [`Config::binaries`] exist.
    ///
    /// Defaults to [`Config::out_dir`].
    #[serde(default, alias = "binaries-dir", alias = "binaries_dir")]
    pub binaries_dir: Option<PathBuf>,
    /// The target triple we are packaging for. This mainly affects [`Config::external_binaries`].
    ///
    /// Defaults to the current OS target triple.
    #[serde(alias = "target-triple", alias = "target_triple")]
    pub target_triple: Option<String>,
    /// The package's description.
    pub description: Option<String>,
    /// The app's long description.
    #[serde(alias = "long-description", alias = "long_description")]
    pub long_description: Option<String>,
    /// The package's homepage.
    pub homepage: Option<String>,
    /// The package's authors.
    #[serde(default)]
    pub authors: Option<Vec<String>>,
    /// The app's publisher. Defaults to the second element in [`Config::identifier`](Config::identifier) string.
    /// Currently maps to the Manufacturer property of the Windows Installer.
    pub publisher: Option<String>,
    /// A path to the license file.
    #[serde(alias = "license-file", alias = "license_file")]
    pub license_file: Option<PathBuf>,
    /// The app's copyright.
    pub copyright: Option<String>,
    /// The app's category.
    pub category: Option<AppCategory>,
    /// The app's icon list. Supports glob patterns.
    pub icons: Option<Vec<String>>,
    /// The file associations
    #[serde(alias = "file-associations", alias = "file_associations")]
    pub file_associations: Option<Vec<FileAssociation>>,
    /// Deep-link protocols.
    #[serde(alias = "deep-link-protocols", alias = "deep_link_protocols")]
    pub deep_link_protocols: Option<Vec<DeepLinkProtocol>>,
    /// The app's resources to package. This a list of either a glob pattern, path to a file, path to a directory
    /// or an object of `src` and `target` paths. In the case of using an object,
    /// the `src` could be either a glob pattern, path to a file, path to a directory,
    /// and the `target` is a path inside the final resources folder in the installed package.
    ///
    /// ## Format-specific:
    ///
    /// - **[PackageFormat::Nsis] / [PackageFormat::Wix]**: The resources are placed next to the executable in the root of the packager.
    /// - **[PackageFormat::Deb]**: The resources are placed in `usr/lib` of the package.
    pub resources: Option<Vec<Resource>>,
    /// Paths to external binaries to add to the package.
    ///
    /// The path specified should not include `-<target-triple><.exe>` suffix,
    /// it will be auto-added when by the packager when reading these paths,
    /// so the actual binary name should have the target platform's target triple appended,
    /// as well as `.exe` for Windows.
    ///
    /// For example, if you're packaging an external binary called `sqlite3`, the packager expects
    /// a binary named `sqlite3-x86_64-unknown-linux-gnu` on linux,
    /// and `sqlite3-x86_64-pc-windows-gnu.exe` on windows.
    ///
    /// If you are building a universal binary for MacOS, the packager expects
    /// your external binary to also be universal, and named after the target triple,
    /// e.g. `sqlite3-universal-apple-darwin`. See
    /// <https://developer.apple.com/documentation/apple-silicon/building-a-universal-macos-binary>
    #[serde(alias = "external-binaries", alias = "external_binaries")]
    pub external_binaries: Option<Vec<PathBuf>>,
    /// Windows-specific configuration.
    pub windows: Option<WindowsConfig>,
    /// MacOS-specific configuration.
    pub macos: Option<MacOsConfig>,
    /// Debian-specific configuration.
    pub deb: Option<DebianConfig>,
    /// AppImage configuration.
    pub appimage: Option<AppImageConfig>,
    /// Pacman configuration.
    pub pacman: Option<PacmanConfig>,
    /// WiX configuration.
    pub wix: Option<WixConfig>,
    /// Nsis configuration.
    pub nsis: Option<NsisConfig>,
    /// Dmg configuration.
    pub dmg: Option<DmgConfig>,
}

impl Config {
    /// Creates a new [`ConfigBuilder`].
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Returns the [windows](Config::windows) specific configuration.
    pub fn windows(&self) -> Option<&WindowsConfig> {
        self.windows.as_ref()
    }

    /// Returns the [macos](Config::macos) specific configuration.
    pub fn macos(&self) -> Option<&MacOsConfig> {
        self.macos.as_ref()
    }

    /// Returns the [nsis](Config::nsis) specific configuration.
    pub fn nsis(&self) -> Option<&NsisConfig> {
        self.nsis.as_ref()
    }

    /// Returns the [wix](Config::wix) specific configuration.
    pub fn wix(&self) -> Option<&WixConfig> {
        self.wix.as_ref()
    }

    /// Returns the [debian](Config::deb) specific configuration.
    pub fn deb(&self) -> Option<&DebianConfig> {
        self.deb.as_ref()
    }

    /// Returns the [appimage](Config::appimage) specific configuration.
    pub fn appimage(&self) -> Option<&AppImageConfig> {
        self.appimage.as_ref()
    }

    /// Returns the [pacman](Config::pacman) specific configuration.
    pub fn pacman(&self) -> Option<&PacmanConfig> {
        self.pacman.as_ref()
    }

    /// Returns the [dmg](Config::dmg) specific configuration.
    pub fn dmg(&self) -> Option<&DmgConfig> {
        self.dmg.as_ref()
    }

    /// Returns the target triple of this config, if not set, fallsback to the current OS target triple.
    pub fn target_triple(&self) -> String {
        self.target_triple.clone().unwrap_or_else(|| {
            util::target_triple().expect("Failed to detect current target triple")
        })
    }

    /// Returns the architecture for the package to be built (e.g. "arm", "x86" or "x86_64").
    pub fn target_arch(&self) -> crate::Result<&str> {
        let target = self.target_triple();
        Ok(if target.starts_with("x86_64") {
            "x86_64"
        } else if target.starts_with('i') {
            "x86"
        } else if target.starts_with("arm") {
            "arm"
        } else if target.starts_with("aarch64") {
            "aarch64"
        } else if target.starts_with("universal") {
            "universal"
        } else {
            return Err(crate::Error::UnexpectedTargetTriple(target));
        })
    }

    /// Returns the path to the specified binary.
    pub fn binary_path(&self, binary: &Binary) -> PathBuf {
        if binary.path.is_absolute() {
            binary.path.clone()
        } else {
            self.binaries_dir().join(&binary.path)
        }
    }

    /// Returns the package identifier. Defaults an empty string.
    pub fn identifier(&self) -> &str {
        self.identifier.as_deref().unwrap_or("")
    }

    /// Returns the package publisher.
    /// Defaults to the second element in [`Config::identifier`](Config::identifier()).
    pub fn publisher(&self) -> String {
        self.publisher.clone().unwrap_or_else(|| {
            self.identifier()
                .split('.')
                .nth(1)
                .unwrap_or(self.identifier())
                .into()
        })
    }

    /// Returns the out dir. Defaults to the current directory.
    pub fn out_dir(&self) -> PathBuf {
        if self.out_dir.as_os_str().is_empty() {
            std::env::current_dir().expect("failed to resolve cwd")
        } else if self.out_dir.exists() {
            dunce::canonicalize(&self.out_dir).unwrap_or_else(|_| self.out_dir.clone())
        } else {
            std::fs::create_dir_all(&self.out_dir).expect("failed to create output directory");
            self.out_dir.clone()
        }
    }

    /// Returns the binaries dir. Defaults to [`Self::out_dir`] if [`Self::binaries_dir`] is not set.
    pub fn binaries_dir(&self) -> PathBuf {
        if let Some(path) = &self.binaries_dir {
            dunce::canonicalize(path).unwrap_or_else(|_| path.clone())
        } else {
            self.out_dir()
        }
    }

    /// Returns the main binary.
    pub fn main_binary(&self) -> crate::Result<&Binary> {
        self.binaries
            .iter()
            .find(|bin| bin.main)
            .ok_or_else(|| crate::Error::MainBinaryNotFound)
    }

    /// Returns the main binary name.
    pub fn main_binary_name(&self) -> crate::Result<String> {
        self.binaries
            .iter()
            .find(|bin| bin.main)
            .map(|b| b.path.file_stem().unwrap().to_string_lossy().into_owned())
            .ok_or_else(|| crate::Error::MainBinaryNotFound)
    }

    /// Returns all icons path.
    pub fn icons(&self) -> crate::Result<Option<Vec<PathBuf>>> {
        let Some(patterns) = &self.icons else {
            return Ok(None);
        };
        let mut paths = Vec::new();
        for pattern in patterns {
            for icon_path in glob::glob(pattern)? {
                paths.push(icon_path?);
            }
        }
        Ok(Some(paths))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedResource {
    pub src: PathBuf,
    pub target: PathBuf,
}

impl Config {
    #[inline]
    pub(crate) fn resources_from_dir(
        src_dir: &Path,
        target_dir: &Path,
    ) -> crate::Result<Vec<ResolvedResource>> {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(src_dir) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let relative = path.relative_to(src_dir)?.to_path("");
                let resource = ResolvedResource {
                    src: dunce::canonicalize(path)?,
                    target: target_dir.join(relative),
                };
                out.push(resource);
            }
        }
        Ok(out)
    }

    #[inline]
    pub(crate) fn resources_from_glob(glob: &str) -> crate::Result<Vec<ResolvedResource>> {
        let mut out = Vec::new();
        for src in glob::glob(glob)? {
            let src = dunce::canonicalize(src?)?;
            let target = PathBuf::from(src.file_name().unwrap_or_default());
            out.push(ResolvedResource { src, target })
        }
        Ok(out)
    }

    pub(crate) fn resources(&self) -> crate::Result<Vec<ResolvedResource>> {
        if let Some(resources) = &self.resources {
            let mut out = Vec::new();
            for r in resources {
                match r {
                    Resource::Single(src) => {
                        let src_dir = PathBuf::from(src);
                        if src_dir.is_dir() {
                            let target_dir = Path::new(src_dir.file_name().unwrap_or_default());
                            out.extend(Self::resources_from_dir(&src_dir, target_dir)?);
                        } else {
                            out.extend(Self::resources_from_glob(src)?);
                        }
                    }
                    Resource::Mapped { src, target } => {
                        let src_path = PathBuf::from(src);
                        let target_dir = sanitize_path(target);
                        if src_path.is_dir() {
                            out.extend(Self::resources_from_dir(&src_path, &target_dir)?);
                        } else if src_path.is_file() {
                            out.push(ResolvedResource {
                                src: dunce::canonicalize(src_path)?,
                                target: sanitize_path(target),
                            });
                        } else {
                            let globbed_res = Self::resources_from_glob(src)?;
                            let retargetd_res = globbed_res.into_iter().map(|mut r| {
                                r.target = target_dir.join(r.target);
                                r
                            });
                            out.extend(retargetd_res);
                        }
                    }
                }
            }

            Ok(out)
        } else {
            Ok(vec![])
        }
    }

    #[allow(unused)]
    pub(crate) fn find_ico(&self) -> crate::Result<Option<PathBuf>> {
        let icon = self
            .icons()?
            .as_ref()
            .and_then(|icons| {
                icons
                    .iter()
                    .find(|i| PathBuf::from(i).extension().and_then(|s| s.to_str()) == Some("ico"))
                    .or_else(|| {
                        icons.iter().find(|i| {
                            PathBuf::from(i).extension().and_then(|s| s.to_str()) == Some("png")
                        })
                    })
            })
            .map(PathBuf::from);
        Ok(icon)
    }

    #[allow(unused)]
    pub(crate) fn copy_resources(&self, path: &Path) -> crate::Result<()> {
        for resource in self.resources()? {
            let dest = path.join(resource.target);
            std::fs::create_dir_all(
                dest.parent()
                    .ok_or_else(|| crate::Error::ParentDirNotFound(dest.to_path_buf()))?,
            )?;
            std::fs::copy(resource.src, dest)?;
        }
        Ok(())
    }

    #[allow(unused)]
    pub(crate) fn copy_external_binaries(&self, path: &Path) -> crate::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        if let Some(external_binaries) = &self.external_binaries {
            let cwd = std::env::current_dir()?;
            let target_triple = self.target_triple();
            for src in external_binaries {
                let file_name = src
                    .file_name()
                    .ok_or_else(|| crate::Error::FailedToExtractFilename(src.clone()))?
                    .to_string_lossy();
                #[cfg(windows)]
                let src = src.with_file_name(format!("{file_name}-{target_triple}.exe"));
                #[cfg(not(windows))]
                let src = src.with_file_name(format!("{file_name}-{target_triple}"));
                #[cfg(windows)]
                let dest = path.join(format!("{file_name}.exe"));
                #[cfg(not(windows))]
                let dest = path.join(&*file_name);
                std::fs::copy(src, &dest)?;
                paths.push(dest);
            }
        }

        Ok(paths)
    }
}

fn sanitize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut dest = PathBuf::new();
    for c in path.as_ref().components() {
        if let std::path::Component::Normal(s) = c {
            dest.push(s)
        }
    }
    dest
}

fn default_true() -> bool {
    true
}
