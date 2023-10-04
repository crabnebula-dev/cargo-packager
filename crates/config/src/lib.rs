// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Config types for [`cargo-packager`](https://docs.rs/cargo-packager).

#![deny(missing_docs)]

use std::{
    collections::HashMap,
    fmt::{self, Display},
    path::PathBuf,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod category;
pub use category::AppCategory;

/// The type of the package we're packaging.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", value(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum PackageFormat {
    /// All available package formats for the current platform.
    ///
    /// See [`PackageFormat::platform_all`]
    All,
    /// The default list of package formats for the current platform.
    ///
    /// See [`PackageFormat::platform_default`]
    Default,
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

impl Display for PackageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

impl PackageFormat {
    /// Maps a short name to a [PackageFormat].
    /// Possible values are "deb", "ios", "wix", "app", "rpm", "appimage", "dmg".
    pub fn from_short_name(name: &str) -> Option<PackageFormat> {
        // Other types we may eventually want to support: apk.
        match name {
            "app" => Some(PackageFormat::App),
            "dmg" => Some(PackageFormat::Dmg),
            "wix" => Some(PackageFormat::Wix),
            "nsis" => Some(PackageFormat::Nsis),
            "deb" => Some(PackageFormat::Deb),
            "appimage" => Some(PackageFormat::AppImage),
            _ => None,
        }
    }

    /// Gets the short name of this [PackageFormat].
    pub fn short_name(&self) -> &'static str {
        match *self {
            PackageFormat::All => "all",
            PackageFormat::Default => "default",
            PackageFormat::App => "app",
            PackageFormat::Dmg => "dmg",
            PackageFormat::Wix => "wix",
            PackageFormat::Nsis => "nsis",
            PackageFormat::Deb => "deb",
            PackageFormat::AppImage => "appimage",
        }
    }

    /// Gets the list of the possible package types on the current OS.
    ///
    /// - **macOS**: App, Dmg
    /// - **Windows**: Nsis, Wix
    /// - **Linux**: Deb, AppImage
    pub fn platform_all() -> &'static [PackageFormat] {
        &[
            #[cfg(target_os = "macos")]
            PackageFormat::App,
            #[cfg(target_os = "macos")]
            PackageFormat::Dmg,
            #[cfg(target_os = "windows")]
            PackageFormat::Wix,
            #[cfg(target_os = "windows")]
            PackageFormat::Nsis,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Deb,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::AppImage,
        ]
    }

    /// Returns the default list of targets this platform
    ///
    /// - **macOS**: App, Dmg
    /// - **Windows**: Nsis
    /// - **Linux**: Deb, AppImage
    pub fn platform_default() -> &'static [PackageFormat] {
        &[
            #[cfg(target_os = "macos")]
            PackageFormat::App,
            #[cfg(target_os = "macos")]
            PackageFormat::Dmg,
            #[cfg(target_os = "windows")]
            PackageFormat::Nsis,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::Deb,
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            PackageFormat::AppImage,
        ]
    }

    /// Gets a number representing priority which used to sort package types
    /// in an order that guarantees that if a certain package type
    /// depends on another (like Dmg depending on MacOsBundle), the dependency
    /// will be built first
    ///
    /// The lower the number, the higher the priority
    pub fn priority(&self) -> u32 {
        match self {
            PackageFormat::All => 0,
            PackageFormat::Default => 0,
            PackageFormat::App => 0,
            PackageFormat::Wix => 0,
            PackageFormat::Nsis => 0,
            PackageFormat::Deb => 0,
            PackageFormat::AppImage => 0,
            PackageFormat::Dmg => 1,
        }
    }
}

/// **macOS-only**. Corresponds to CFBundleTypeRole
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileAssociation {
    /// File extensions to associate with this app. e.g. 'png'
    pub ext: Vec<String>,
    /// The name. Maps to `CFBundleTypeName` on macOS. Default to the first item in `ext`
    pub name: Option<String>,
    /// The association description. **Windows-only**. It is displayed on the `Type` column on Windows Explorer.
    pub description: Option<String>,
    /// The app’s role with respect to the type. Maps to `CFBundleTypeRole` on macOS.
    #[serde(default)]
    pub role: BundleTypeRole,
    /// The mime-type e.g. 'image/png' or 'text/plain'. Linux-only.
    #[serde(alias = "mime-type", alias = "mime_type")]
    pub mime_type: Option<String>,
}

/// The Linux debian configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DebianConfig {
    /// the list of debian dependencies.
    pub depends: Option<Vec<String>>,
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
    /// List of custom files to add to the deb package.
    /// Maps a dir/file to a dir/file inside the debian package.
    pub files: Option<HashMap<String, String>>,
}

/// The Linux AppImage configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppImageConfig {
    /// List of libs that exist in `/usr/lib*` to be include in the final AppImage.
    /// The libs will be searched for using the command
    /// `find -L /usr/lib* -name <libname>`
    pub libs: Option<Vec<String>>,
    /// List of binary paths to include in the final AppImage.
    /// For example, if you want `xdg-open`, you'd specify `/usr/bin/xdg-open`
    pub bins: Option<Vec<String>>,
    /// Hashmap of [`linuxdeploy`](https://github.com/linuxdeploy/linuxdeploy)
    /// plugin name and its URL to be downloaded and executed while packaing the appimage.
    /// For example, if you want to use the
    /// [`gtk`](https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh) plugin,
    /// you'd specify `gtk` as the key and its url as the value.
    #[serde(alias = "linuxdeploy-plugins", alias = "linuxdeploy_plugins")]
    pub linuxdeploy_plugins: Option<HashMap<String, String>>,
}

/// The macOS configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    /// Provider short name for notarization.
    #[serde(alias = "provider-short-name", alias = "provider_short_name")]
    pub provider_short_name: Option<String>,
    /// Path to the entitlements.plist file.
    pub entitlements: Option<String>,
    /// Path to the Info.plist file for the package.
    #[serde(alias = "info-plist-path", alias = "info_plist_path")]
    pub info_plist_path: Option<PathBuf>,
}

/// Configuration for a target language for the WiX build.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WixLanguageConfig {
    /// The path to a locale (`.wxl`) file. See <https://wixtoolset.org/documentation/manual/v3/howtos/ui_and_localization/build_a_localized_version.html>.
    #[serde(alias = "locale-Path", alias = "locale_Path")]
    pub locale_path: Option<PathBuf>,
}

/// The languages to build using WiX.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct WixLanguages(pub Vec<(String, WixLanguageConfig)>);

impl Default for WixLanguages {
    fn default() -> Self {
        Self(vec![("en-US".into(), Default::default())])
    }
}

/// The wix format configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WixConfig {
    /// The app languages to build. See <https://docs.microsoft.com/en-us/windows/win32/msi/localizing-the-error-and-actiontext-tables>.
    #[serde(default)]
    pub languages: WixLanguages,
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

/// Install Modes for the NSIS installer.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NsisCompression {
    /// ZLIB uses the deflate algorithm, it is a quick and simple method. With the default compression level it uses about 300 KB of memory.
    Zlib,
    /// BZIP2 usually gives better compression ratios than ZLIB, but it is a bit slower and uses more memory. With the default compression level it uses about 4 MB of memory.
    Bzip2,
    /// LZMA (default) is a new compression method that gives very good compression ratios. The decompression speed is high (10-20 MB/s on a 2 GHz CPU), the compression speed is lower. The memory size that will be used for decompression is the dictionary size plus a few KBs, the default is 8 MB.
    Lzma,
}

/// The NSIS format configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

/// The Windows configuration.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsConfig {
    /// The file digest algorithm to use for creating file signatures. Required for code signing. SHA-256 is recommended.
    #[serde(alias = "digest-algorithim", alias = "digest_algorithim")]
    pub digest_algorithm: Option<String>,
    /// The SHA1 hash of the signing certificate.
    #[serde(alias = "certificate-thumbprint", alias = "certificate_thumbprint")]
    pub certificate_thumbprint: Option<String>,
    /// Server to use during timestamping.
    #[serde(alias = "timestamp-url", alias = "timestamp_url")]
    pub timestamp_url: Option<String>,
    /// Whether to use Time-Stamp Protocol (TSP, a.k.a. RFC 3161) for the timestamp server. Your code signing provider may
    /// use a TSP timestamp server, like e.g. SSL.com does. If so, enable TSP by setting to true.
    #[serde(default)]
    pub tsp: bool,
    /// Validates a second app installation, blocking the user from installing an older version if set to `false`.
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
}

fn default_true() -> bool {
    true
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            digest_algorithm: None,
            certificate_thumbprint: None,
            timestamp_url: None,
            tsp: false,
            allow_downgrades: true,
        }
    }
}

/// An enum representing the available verbosity levels of the logger.
#[derive(Deserialize, Serialize)]
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, JsonSchema)]
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
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binary {
    /// File name and without `.exe` on Windows
    pub filename: String,
    /// Whether this is the main binary or not
    #[serde(default)]
    pub main: bool,
}

/// A path to a resource (with optional glob pattern)
/// or an object of `src` and `target` paths.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
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
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
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
#[derive(Deserialize, Serialize, Default, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// Whether this config is enabled or not. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// The JSON schema for the config.
    ///
    /// Setting this field has no effect, this just exists so
    /// we can parse the JSON correct when it has `$schema` field set.
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    /// The app name, this is just an identifier that could be used
    /// to filter which app to package using `--pacakges` cli arg when there is multiple apps in the
    /// workspace or in the same config.
    ///
    /// This field resembles, the `name` field in `Cargo.toml` and `package.json`
    ///
    /// If `unset`, the CLI will try to auto-detect it from `Cargo.toml` or
    /// `package.json` otherwise, it will keep it as null.
    pub name: Option<String>,
    /// Specify a command to run before starting to package an application.
    ///
    /// This runs only once.
    #[serde(
        default,
        alias = "before-packaging-command",
        alias = "before_packaging_command"
    )]
    pub before_packaging_command: Option<HookCommand>,
    /// Specify a command to run before packaging each format for an application.
    ///
    /// This will run multiple times depending on the formats specifed.
    #[serde(
        default,
        alias = "before-each-package-command",
        alias = "before_each_package_command"
    )]
    pub before_each_package_command: Option<HookCommand>,
    /// The log level.
    #[serde(alias = "log-level", alias = "log_level")]
    pub log_level: Option<LogLevel>,
    /// The package types we're creating.
    ///
    /// if not present, we'll use the PackageType list for the target OS.
    pub formats: Option<Vec<PackageFormat>>,
    /// The directory where the `binaries` exist and where the packages will be placed.
    #[serde(default, alias = "out-dir", alias = "out_dir")]
    pub out_dir: PathBuf,
    /// The target triple. Defaults to the current OS target triple.
    #[serde(alias = "target-triple", alias = "target_triple")]
    pub target_triple: Option<String>,
    /// the package's product name, for example "My Awesome App".
    #[serde(default, alias = "product-name", alias = "product_name")]
    pub product_name: String,
    /// the package's version.
    #[serde(default)]
    pub version: String,
    /// the package's description.
    pub description: Option<String>,
    /// the app's long description.
    #[serde(alias = "long-description", alias = "long_description")]
    pub long_description: Option<String>,
    /// the package's homepage.
    pub homepage: Option<String>,
    /// the package's authors.
    #[serde(default)]
    pub authors: Vec<String>,
    /// the app's identifier.
    pub identifier: Option<String>,
    /// The app's publisher. Defaults to the second element in the identifier string.
    /// Currently maps to the Manufacturer property of the Windows Installer.
    pub publisher: Option<String>,
    /// A path to the license file.
    #[serde(alias = "license-file", alias = "license_file")]
    pub license_file: Option<PathBuf>,
    /// the app's copyright.
    pub copyright: Option<String>,
    /// the app's category.
    pub category: Option<AppCategory>,
    /// the app's icon list.
    pub icons: Option<Vec<String>>,
    /// the binaries to package.
    #[serde(default)]
    pub binaries: Vec<Binary>,
    /// the file associations
    #[serde(alias = "file-associations", alias = "file_associations")]
    pub file_associations: Option<Vec<FileAssociation>>,
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
    /// External binaries to add to the package.
    ///
    /// Note that each binary name should have the target platform's target triple appended,
    /// as well as `.exe` for Windows.
    /// For example, if you're packaging a sidecar called `sqlite3`, the packager expects
    /// a binary named `sqlite3-x86_64-unknown-linux-gnu` on linux,
    /// and `sqlite3-x86_64-pc-windows-gnu.exe` on windows.
    ///
    /// If you are building a universal binary for MacOS, the packager expects
    /// your external binary to also be universal, and named after the target triple,
    /// e.g. `sqlite3-universal-apple-darwin`. See
    /// <https://developer.apple.com/documentation/apple-silicon/building-a-universal-macos-binary>
    #[serde(alias = "external-binaries", alias = "external_binaries")]
    pub external_binaries: Option<Vec<String>>,
    /// Debian-specific settings.
    pub deb: Option<DebianConfig>,
    /// Debian-specific settings.
    pub appimage: Option<AppImageConfig>,
    /// WiX configuration.
    pub wix: Option<WixConfig>,
    /// Nsis configuration.
    pub nsis: Option<NsisConfig>,
    /// MacOS-specific settings.
    pub macos: Option<MacOsConfig>,
    /// Windows-specific settings.
    pub windows: Option<WindowsConfig>,
}
