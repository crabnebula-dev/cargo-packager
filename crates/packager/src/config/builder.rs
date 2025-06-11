use std::path::PathBuf;

use crate::{Config, PackageFormat};

use super::{
    AppImageConfig, Binary, DebianConfig, FileAssociation, HookCommand, LogLevel, MacOsConfig,
    NsisConfig, PacmanConfig, Resource, WindowsConfig, WixConfig, RpmConfig,
};

/// A builder type for [`Config`].
#[derive(Default)]
pub struct ConfigBuilder(Config);

impl ConfigBuilder {
    /// Creates a new config builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the config used by this builder.
    pub fn config(&self) -> &Config {
        &self.0
    }

    /// Sets [`Config::product_name`].
    pub fn product_name<S: Into<String>>(mut self, product_name: S) -> Self {
        self.0.product_name = product_name.into();
        self
    }

    /// Sets [`Config::version`].
    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.0.version = version.into();
        self
    }

    /// Sets [`Config::binaries`].
    pub fn binaries<I: IntoIterator<Item = Binary>>(mut self, binaries: I) -> Self {
        self.0.binaries = binaries.into_iter().collect();
        self
    }

    /// Sets [`Config::identifier`].
    pub fn identifier<S: Into<String>>(mut self, identifier: S) -> Self {
        self.0.identifier.replace(identifier.into());
        self
    }

    /// Sets [`Config::before_packaging_command`].
    pub fn before_packaging_command(mut self, command: HookCommand) -> Self {
        self.0.before_packaging_command.replace(command);
        self
    }

    /// Sets [`Config::before_each_package_command`].
    pub fn before_each_package_command(mut self, command: HookCommand) -> Self {
        self.0.before_each_package_command.replace(command);
        self
    }

    /// Sets [`Config::log_level`].
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.0.log_level.replace(level);
        self
    }

    /// Sets [`Config::formats`].
    pub fn formats<I: IntoIterator<Item = PackageFormat>>(mut self, formats: I) -> Self {
        self.0.formats = Some(formats.into_iter().collect());
        self
    }

    /// Sets [`Config::out_dir`].
    pub fn out_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.0.out_dir = path.into();
        self
    }

    /// Sets [`Config::target_triple`].
    pub fn target_triple<S: Into<String>>(mut self, target_triple: S) -> Self {
        self.0.target_triple.replace(target_triple.into());
        self
    }

    /// Sets [`Config::description`].
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.0.description.replace(description.into());
        self
    }

    /// Sets [`Config::long_description`].
    pub fn long_description<S: Into<String>>(mut self, long_description: S) -> Self {
        self.0.long_description.replace(long_description.into());
        self
    }

    /// Sets [`Config::homepage`].
    pub fn homepage<S: Into<String>>(mut self, homepage: S) -> Self {
        self.0.homepage.replace(homepage.into());
        self
    }

    /// Sets [`Config::authors`].
    pub fn authors<I, S>(mut self, authors: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0
            .authors
            .replace(authors.into_iter().map(Into::into).collect());
        self
    }

    /// Sets [`Config::publisher`].
    pub fn publisher<S: Into<String>>(mut self, publisher: S) -> Self {
        self.0.publisher.replace(publisher.into());
        self
    }

    /// Sets [`Config::license_file`].
    pub fn license_file<P: Into<PathBuf>>(mut self, license_file: P) -> Self {
        self.0.license_file.replace(license_file.into());
        self
    }

    /// Sets [`Config::copyright`].
    pub fn copyright<S: Into<String>>(mut self, copyright: S) -> Self {
        self.0.copyright.replace(copyright.into());
        self
    }

    /// Sets [`Config::icons`].
    pub fn icons<I, S>(mut self, icons: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0
            .icons
            .replace(icons.into_iter().map(Into::into).collect());
        self
    }

    /// Sets [`Config::file_associations`].
    pub fn file_associations<I: IntoIterator<Item = FileAssociation>>(
        mut self,
        file_associations: I,
    ) -> Self {
        self.0
            .file_associations
            .replace(file_associations.into_iter().collect());
        self
    }

    /// Sets [`Config::resources`].
    pub fn resources<I: IntoIterator<Item = Resource>>(mut self, resources: I) -> Self {
        self.0.resources.replace(resources.into_iter().collect());
        self
    }

    /// Sets [`Config::external_binaries`].
    pub fn external_binaries<I, P>(mut self, external_binaries: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.0
            .external_binaries
            .replace(external_binaries.into_iter().map(Into::into).collect());
        self
    }

    /// Set the [Windows](Config::windows) specific configuration.
    pub fn windows(mut self, windows: WindowsConfig) -> Self {
        self.0.windows.replace(windows);
        self
    }

    /// Set the [MacOS](Config::macos) specific configuration.
    pub fn macos(mut self, macos: MacOsConfig) -> Self {
        self.0.macos.replace(macos);
        self
    }

    /// Set the [WiX](Config::wix) specific configuration.
    pub fn wix(mut self, wix: WixConfig) -> Self {
        self.0.wix.replace(wix);
        self
    }

    /// Set the [Nsis](Config::nsis) specific configuration.
    pub fn nsis(mut self, nsis: NsisConfig) -> Self {
        self.0.nsis.replace(nsis);
        self
    }

    /// Set the [Debian](Config::deb) specific configuration.
    pub fn deb(mut self, deb: DebianConfig) -> Self {
        self.0.deb.replace(deb);
        self
    }

    /// Set the [Appimage](Config::appimage) specific configuration.
    pub fn appimage(mut self, appimage: AppImageConfig) -> Self {
        self.0.appimage.replace(appimage);
        self
    }

    /// Set the [Pacman](Config::pacman) specific configuration.
    pub fn pacman(mut self, pacman: PacmanConfig) -> Self {
        self.0.pacman.replace(pacman);
        self
    }

    /// Set the [Redhat](Config::rpm) specific configuration.
    pub fn rpm(mut self, rpm: RpmConfig) -> Self {
        self.0.rpm.replace(rpm);
        self
    }
}
