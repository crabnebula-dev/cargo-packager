use napi::{Result, Status};

use cargo_packager_resource_resolver::PackageFormat as ResolverPackageFormat;

/// Types of supported packages by [`@crabnebula/packager`](https://www.npmjs.com/package/@crabnebula/packager)
#[derive(Debug, Eq, PartialEq)]
#[napi_derive::napi(string_enum)]
pub enum PackageFormat {
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

impl From<PackageFormat> for ResolverPackageFormat {
    fn from(value: PackageFormat) -> Self {
        match value {
            PackageFormat::App => ResolverPackageFormat::App,
            PackageFormat::Dmg => ResolverPackageFormat::Dmg,
            PackageFormat::Wix => ResolverPackageFormat::Wix,
            PackageFormat::Nsis => ResolverPackageFormat::Nsis,
            PackageFormat::Deb => ResolverPackageFormat::Deb,
            PackageFormat::AppImage => ResolverPackageFormat::AppImage,
        }
    }
}

/// Retrieve the resource path of your app, packaged with cargo packager.
#[napi_derive::napi]
pub fn resources_dir(package_format: PackageFormat) -> Result<String> {
    cargo_packager_resource_resolver::resources_dir(package_format.into())
        .map_err(|e| napi::Error::new(Status::GenericFailure, e.to_string()))
        .map(|p| dunce::simplified(&p).to_string_lossy().to_string())
}
