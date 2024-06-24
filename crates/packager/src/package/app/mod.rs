// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::path::{Path, PathBuf};

use super::Context;
use crate::{config::Config, util};

#[cfg(target_os = "macos")]
use crate::{
    codesign::macos::{self as codesign, SignTarget},
    shell::CommandExt,
};

#[tracing::instrument(level = "trace")]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let Context { config, .. } = ctx;
    // we should use the bundle name (App name) as a MacOS standard.
    // version or platform shouldn't be included in the App name.
    let app_product_name = format!("{}.app", config.product_name);
    let app_bundle_path = config.out_dir().join(&app_product_name);

    if app_bundle_path.exists() {
        std::fs::remove_dir_all(&app_bundle_path)?;
    }

    tracing::info!(
        "Packaging {} ({})",
        app_product_name,
        app_bundle_path.display()
    );

    let contents_directory = app_bundle_path.join("Contents");
    std::fs::create_dir_all(&contents_directory)?;

    let resources_dir = contents_directory.join("Resources");
    let bin_dir = contents_directory.join("MacOS");
    std::fs::create_dir_all(&bin_dir)?;

    #[cfg(target_os = "macos")]
    let mut sign_paths = std::collections::BinaryHeap::new();

    let bundle_icon_file = util::create_icns_file(&resources_dir, config)?;

    tracing::debug!("Creating Info.plist");
    create_info_plist(&contents_directory, bundle_icon_file, config)?;

    tracing::debug!("Copying frameworks");
    let _framework_paths = copy_frameworks_to_bundle(&contents_directory, config)?;

    #[cfg(target_os = "macos")]
    sign_paths.extend(
        _framework_paths
            .into_iter()
            .filter(|p| {
                let ext = p.extension();
                ext == Some(std::ffi::OsStr::new("framework"))
            })
            .map(|path| SignTarget {
                path,
                is_native_binary: false,
            }),
    );

    tracing::debug!("Copying resources");
    config.copy_resources(&resources_dir)?;

    tracing::debug!("Copying external binaries");
    config.copy_external_binaries(&bin_dir)?;
    tracing::debug!("Copying binaries");
    for bin in &config.binaries {
        let bin_path = config.binary_path(bin);
        let dest_path = bin_dir.join(bin.path.file_name().unwrap());
        std::fs::copy(&bin_path, &dest_path)?;
    }

    // All dylib files and native executables should be signed manually
    // It is highly discouraged by Apple to use the --deep codesign parameter in larger projects.
    // https://developer.apple.com/forums/thread/129980

    // Find all files in the app bundle
    let files = walkdir::WalkDir::new(&app_bundle_path)
        .into_iter()
        .flatten()
        .map(|dir| dir.into_path());

    // Filter all files for Mach-O headers. This will target all .dylib and native executable files
    for file in files {
        let metadata = match std::fs::metadata(&file) {
            Ok(f) => f,
            Err(err) => {
                tracing::warn!("Failed to get metadata for {}: {err}, this file will not be scanned for Mach-O header!", file.display());
                continue;
            }
        };

        // ignore folders and files that do not include at least the header size
        if !metadata.is_file() || metadata.len() < 4 {
            continue;
        }

        let mut open_file = match std::fs::File::open(&file) {
            Ok(f) => f,
            Err(err) => {
                tracing::warn!("Failed to open {} for reading: {err}, this file will not be scanned for Mach-O header!", file.display());
                continue;
            }
        };

        let mut buffer = [0; 4];
        std::io::Read::read_exact(&mut open_file, &mut buffer)?;

        const MACH_O_MAGIC_NUMBERS: [u32; 5] =
            [0xfeedface, 0xfeedfacf, 0xcafebabe, 0xcefaedfe, 0xcffaedfe];

        let magic = u32::from_be_bytes(buffer);

        let is_mach = MACH_O_MAGIC_NUMBERS.contains(&magic);
        if !is_mach {
            continue;
        }

        #[cfg(target_os = "macos")]
        sign_paths.push(SignTarget {
            path: file,
            is_native_binary: true,
        });
    }

    #[cfg(target_os = "macos")]
    if let Some(identity) = config
        .macos()
        .and_then(|macos| macos.signing_identity.as_ref())
    {
        tracing::debug!("Codesigning {}", app_bundle_path.display());
        // Sign frameworks and sidecar binaries first, per apple, signing must be done inside out
        // https://developer.apple.com/forums/thread/701514
        sign_paths.push(SignTarget {
            path: app_bundle_path.clone(),
            is_native_binary: true,
        });

        // Remove extra attributes, which could cause codesign to fail
        // https://developer.apple.com/library/archive/qa/qa1940/_index.html
        remove_extra_attr(&app_bundle_path)?;

        // sign application
        let sign_paths = sign_paths.into_sorted_vec();
        codesign::try_sign(sign_paths, identity, config)?;

        // notarization is required for distribution
        match config
            .macos()
            .and_then(|m| m.notarization_credentials.clone())
            .ok_or(crate::Error::MissingNotarizeAuthVars)
            .or_else(|_| codesign::notarize_auth())
        {
            Ok(auth) => {
                tracing::debug!("Notarizing {}", app_bundle_path.display());
                codesign::notarize(app_bundle_path.clone(), auth, config)?;
            }
            Err(e) => {
                tracing::warn!("Skipping app notarization, {}", e.to_string());
            }
        }
    }

    Ok(vec![app_bundle_path])
}

// Creates the Info.plist file.
#[tracing::instrument(level = "trace")]
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
                .ok_or_else(|| crate::Error::FailedToExtractFilename(path.clone()))?
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
                                    .extensions
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
                                .unwrap_or(&association.extensions[0])
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

    if let Some(protocols) = &config.deep_link_protocols {
        plist.insert(
            "CFBundleURLTypes".into(),
            plist::Value::Array(
                protocols
                    .iter()
                    .map(|protocol| {
                        let mut dict = plist::Dictionary::new();
                        dict.insert(
                            "CFBundleURLSchemes".into(),
                            plist::Value::Array(
                                protocol
                                    .schemes
                                    .iter()
                                    .map(|s| s.to_string().into())
                                    .collect(),
                            ),
                        );
                        dict.insert(
                            "CFBundleURLName".into(),
                            protocol
                                .name
                                .clone()
                                .unwrap_or(format!(
                                    "{} {}",
                                    config.identifier(),
                                    protocol.schemes[0]
                                ))
                                .into(),
                        );
                        dict.insert("CFBundleTypeRole".into(), protocol.role.to_string().into());
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

#[tracing::instrument(level = "trace")]
fn copy_dir(from: &Path, to: &Path) -> crate::Result<()> {
    if !from.exists() {
        return Err(crate::Error::DoesNotExist(from.to_path_buf()));
    }
    if !from.is_dir() {
        return Err(crate::Error::IsNotDirectory(from.to_path_buf()));
    }
    if to.exists() {
        return Err(crate::Error::AlreadyExists(to.to_path_buf()));
    }

    let parent = to
        .parent()
        .ok_or_else(|| crate::Error::ParentDirNotFound(to.to_path_buf()))?;
    std::fs::create_dir_all(parent)?;
    for entry in walkdir::WalkDir::new(from) {
        let entry = entry?;
        debug_assert!(entry.path().starts_with(from));
        let rel_path = entry.path().strip_prefix(from)?;
        let dest_path = to.join(rel_path);
        if entry.file_type().is_symlink() {
            let target = std::fs::read_link(entry.path())?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &dest_path)?;
            #[cfg(windows)]
            {
                if entry.file_type().is_file() {
                    std::os::windows::fs::symlink_file(&target, &dest_path)?;
                } else {
                    std::os::windows::fs::symlink_dir(&target, &dest_path)?;
                }
            }
        } else if entry.file_type().is_dir() {
            std::fs::create_dir(dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

// Copies the framework under `{src_dir}/{framework}.framework` to `{dest_dir}/{framework}.framework`.
#[tracing::instrument(level = "trace")]
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
#[tracing::instrument(level = "trace")]
fn copy_frameworks_to_bundle(
    contents_directory: &Path,
    config: &Config,
) -> crate::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    if let Some(frameworks) = config.macos().and_then(|m| m.frameworks.as_ref()) {
        let dest_dir = contents_directory.join("Frameworks");
        std::fs::create_dir_all(contents_directory)?;

        for framework in frameworks {
            if framework.ends_with(".framework") || framework.ends_with(".app") {
                let src_path = PathBuf::from(framework);
                let src_name = src_path
                    .file_name()
                    .ok_or_else(|| crate::Error::FailedToExtractFilename(src_path.clone()))?;
                let dest_path = dest_dir.join(src_name);
                copy_dir(&src_path, &dest_path)?;
                paths.push(dest_path);
                continue;
            } else if framework.ends_with(".dylib") {
                let src_path = PathBuf::from(&framework);
                if !src_path.exists() {
                    return Err(crate::Error::FrameworkNotFound(framework.to_string()));
                }
                let src_name = src_path
                    .file_name()
                    .ok_or_else(|| crate::Error::FailedToExtractFilename(src_path.clone()))?;
                std::fs::create_dir_all(&dest_dir)?;
                let dest_path = dest_dir.join(src_name);
                std::fs::copy(&src_path, &dest_path)?;
                paths.push(dest_path);
                continue;
            } else if framework.contains('/') {
                return Err(crate::Error::InvalidFramework {
                    framework: framework.to_string(),
                    reason: "framework extension should be either .framework, .dylib or .app",
                });
            }
            if let Some(home_dir) = dirs::home_dir() {
                if copy_framework_from(&dest_dir, framework, &home_dir.join("Library/Frameworks/"))?
                {
                    continue;
                }
            }
            if copy_framework_from(&dest_dir, framework, &PathBuf::from("/Library/Frameworks/"))?
                || copy_framework_from(
                    &dest_dir,
                    framework,
                    &PathBuf::from("/Network/Library/Frameworks/"),
                )?
            {
                continue;
            }

            return Err(crate::Error::FrameworkNotFound(framework.to_string()));
        }
    }

    Ok(paths)
}

#[cfg(target_os = "macos")]
fn remove_extra_attr(app_bundle_path: &Path) -> crate::Result<()> {
    std::process::Command::new("xattr")
        .arg("-cr")
        .arg(app_bundle_path)
        .output_ok()
        .map(|_| ())
        .map_err(crate::Error::FailedToRemoveExtendedAttributes)
}
