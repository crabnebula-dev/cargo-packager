use std::path::{Path, PathBuf};

use crate::{
    config::{Config, ConfigExt, ConfigExtInternal},
    sign, util,
};

pub fn package(config: &Config) -> crate::Result<Vec<PathBuf>> {
    // we should use the bundle name (App name) as a MacOS standard.
    // version or platform shouldn't be included in the App name.
    let app_product_name = format!("{}.app", config.product_name);
    let app_bundle_path = config.out_dir().join(&app_product_name);

    log::info!(action = "Packaging"; "{} ({})", app_product_name, app_bundle_path.display());

    let contents_directory = app_bundle_path.join("Contents");
    std::fs::create_dir_all(&contents_directory)?;

    let resources_dir = contents_directory.join("Resources");
    let bin_dir = contents_directory.join("MacOS");

    let bundle_icon_file = util::create_icns_file(&resources_dir, config)?;

    log::debug!("creating info.plist");
    create_info_plist(&contents_directory, bundle_icon_file, config)?;

    log::debug!("copying frameworks");
    copy_frameworks_to_bundle(&contents_directory, config)?;

    log::debug!("copying resources");
    config.copy_resources(&resources_dir)?;

    log::debug!("copying external binaries");
    config.copy_external_binaries(&bin_dir)?;

    log::debug!("copying binaries");
    copy_binaries_to_bundle(&contents_directory, config)?;

    if let Some(identity) = config
        .macos()
        .and_then(|macos| macos.signing_identity.as_ref())
    {
        sign::try_sign(app_bundle_path.clone(), identity, config, true)?;
        // notarization is required for distribution
        match sign::notarize_auth() {
            Ok(auth) => {
                sign::notarize(app_bundle_path.clone(), auth, config)?;
            }
            Err(e) => {
                log::warn!("skipping app notarization, {}", e.to_string());
            }
        }
    }

    Ok(vec![app_bundle_path])
}

// Copies the app's binaries to the bundle.
fn copy_binaries_to_bundle(contents_directory: &Path, config: &Config) -> crate::Result<()> {
    let bin_dir = contents_directory.join("MacOS");
    std::fs::create_dir_all(&bin_dir)?;

    for bin in &config.binaries {
        let bin_path = config.binary_path(bin);
        std::fs::copy(&bin_path, bin_dir.join(&bin.filename))?;
    }

    Ok(())
}

// Creates the Info.plist file.
fn create_info_plist(
    contents_directory: &Path,
    bundle_icon_file: Option<PathBuf>,
    config: &Config,
) -> crate::Result<()> {
    let format = time::format_description::parse("[year][month][day].[hour][minute][second]")
        .map_err(time::error::Error::from)?;
    let build_number = time::OffsetDateTime::now_utc()
        .format(&format)
        .map_err(time::error::Error::from)?;

    let mut plist = plist::Dictionary::new();
    plist.insert("CFBundleDevelopmentRegion".into(), "English".into());
    plist.insert(
        "CFBundleDisplayName".into(),
        config.product_name.clone().into(),
    );
    plist.insert(
        "CFBundleExecutable".into(),
        config.main_binary_name()?.clone().into(),
    );
    if let Some(path) = bundle_icon_file {
        plist.insert(
            "CFBundleIconFile".into(),
            path.file_name()
                .expect("No file name")
                .to_string_lossy()
                .into_owned()
                .into(),
        );
    }
    plist.insert("CFBundleIdentifier".into(), config.identifier().into());
    plist.insert("CFBundleInfoDictionaryVersion".into(), "6.0".into());
    plist.insert("CFBundleName".into(), config.product_name.clone().into());
    plist.insert("CFBundlePackageType".into(), "APPL".into());
    plist.insert(
        "CFBundleShortVersionString".into(),
        config.version.clone().into(),
    );
    plist.insert("CFBundleVersion".into(), build_number.into());
    plist.insert("CSResourcesFileMapped".into(), true.into());
    if let Some(category) = &config.category {
        plist.insert(
            "LSApplicationCategoryType".into(),
            category.macos_application_category_type().into(),
        );
    }
    if let Some(version) = config
        .macos()
        .and_then(|macos| macos.minimum_system_version.as_deref())
    {
        plist.insert("LSMinimumSystemVersion".into(), version.into());
    }

    if let Some(associations) = &config.file_associations {
        plist.insert(
            "CFBundleDocumentTypes".into(),
            plist::Value::Array(
                associations
                    .iter()
                    .map(|association| {
                        let mut dict = plist::Dictionary::new();
                        dict.insert(
                            "CFBundleTypeExtensions".into(),
                            plist::Value::Array(
                                association
                                    .ext
                                    .iter()
                                    .map(|ext| ext.to_string().into())
                                    .collect(),
                            ),
                        );
                        dict.insert(
                            "CFBundleTypeName".into(),
                            association
                                .name
                                .as_ref()
                                .unwrap_or(&association.ext[0])
                                .to_string()
                                .into(),
                        );
                        dict.insert(
                            "CFBundleTypeRole".into(),
                            association.role.to_string().into(),
                        );
                        plist::Value::Dictionary(dict)
                    })
                    .collect(),
            ),
        );
    }

    plist.insert("LSRequiresCarbon".into(), true.into());
    plist.insert("NSHighResolutionCapable".into(), true.into());
    if let Some(copyright) = &config.copyright {
        plist.insert("NSHumanReadableCopyright".into(), copyright.clone().into());
    }

    if let Some(exception_domain) = config
        .macos()
        .and_then(|macos| macos.exception_domain.clone())
    {
        let mut security = plist::Dictionary::new();
        let mut domain = plist::Dictionary::new();
        domain.insert("NSExceptionAllowsInsecureHTTPLoads".into(), true.into());
        domain.insert("NSIncludesSubdomains".into(), true.into());

        let mut exception_domains = plist::Dictionary::new();
        exception_domains.insert(exception_domain, domain.into());
        security.insert("NSExceptionDomains".into(), exception_domains.into());
        plist.insert("NSAppTransportSecurity".into(), security.into());
    }

    if let Some(user_plist_path) = config
        .macos()
        .and_then(|macos| macos.info_plist_path.as_ref())
    {
        let user_plist = plist::Value::from_file(user_plist_path)?;
        if let Some(dict) = user_plist.into_dictionary() {
            for (key, value) in dict {
                plist.insert(key, value);
            }
        }
    }

    plist::Value::Dictionary(plist).to_file_xml(contents_directory.join("Info.plist"))?;

    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> crate::Result<()> {
    if !from.exists() {
        return Err(crate::Error::AlreadyExists(from.to_path_buf()));
    }
    if !from.is_dir() {
        return Err(crate::Error::IsNotDirectory(from.to_path_buf()));
    }
    if to.exists() {
        return Err(crate::Error::AlreadyExists(to.to_path_buf()));
    }

    let parent = to.parent().expect("No data in parent");
    std::fs::create_dir_all(parent)?;
    for entry in walkdir::WalkDir::new(from) {
        let entry = entry?;
        debug_assert!(entry.path().starts_with(from));
        let rel_path = entry.path().strip_prefix(from)?;
        let dest_path = to.join(rel_path);
        if entry.file_type().is_symlink() {
            let target = std::fs::read_link(entry.path())?;
            std::os::unix::fs::symlink(&target, &dest_path)?;
        } else if entry.file_type().is_dir() {
            std::fs::create_dir(dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

// Copies the framework under `{src_dir}/{framework}.framework` to `{dest_dir}/{framework}.framework`.
fn copy_framework_from(dest_dir: &Path, framework: &str, src_dir: &Path) -> crate::Result<bool> {
    let src_name = format!("{}.framework", framework);
    let src_path = src_dir.join(&src_name);
    if src_path.exists() {
        copy_dir(&src_path, &dest_dir.join(&src_name))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// Copies the macOS application bundle frameworks to the .app
fn copy_frameworks_to_bundle(contents_directory: &Path, config: &Config) -> crate::Result<()> {
    if let Some(frameworks) = config.macos().and_then(|m| m.frameworks.as_ref()) {
        let dest_dir = contents_directory.join("Frameworks");
        std::fs::create_dir_all(contents_directory)?;

        for framework in frameworks {
            if framework.ends_with(".framework") {
                let src_path = PathBuf::from(framework);
                let src_name = src_path
                    .file_name()
                    .expect("Couldn't get framework filename");
                copy_dir(&src_path, &dest_dir.join(src_name))?;
                continue;
            } else if framework.ends_with(".dylib") {
                let src_path = PathBuf::from(&framework);
                if !src_path.exists() {
                    return Err(crate::Error::FrameworkNotFound(framework.to_string()));
                }
                let src_name = src_path.file_name().expect("Couldn't get library filename");
                std::fs::create_dir_all(&dest_dir)?;
                std::fs::copy(&src_path, dest_dir.join(src_name))?;
                continue;
            } else if framework.contains('/') {
                return Err(crate::Error::InvalidFramework {
                    framework.to_string(),
                    reason: "path should have the .framework extension",
                });
            }
            if let Some(home_dir) = dirs::home_dir() {
                if copy_framework_from(
                    &dest_dir,
                    &framework,
                    &home_dir.join("Library/Frameworks/"),
                )? {
                    continue;
                }
            }
            if copy_framework_from(
                &dest_dir,
                &framework,
                &PathBuf::from("/Library/Frameworks/"),
            )? || copy_framework_from(
                &dest_dir,
                &framework,
                &PathBuf::from("/Network/Library/Frameworks/"),
            )? {
                continue;
            }

            return Err(crate::Error::FrameworkNotFound(framework.to_string()));
        }
    }

    Ok(())
}
