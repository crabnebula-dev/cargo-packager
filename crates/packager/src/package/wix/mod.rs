// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use handlebars::{to_json, Handlebars};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Context;
use crate::{
    codesign,
    config::{Config, LogLevel, WixLanguage},
    shell::CommandExt,
    util::{self, download_and_verify, extract_zip, HashAlgorithm},
};

pub const WIX_URL: &str =
    "https://github.com/wixtoolset/wix3/releases/download/wix3112rtm/wix311-binaries.zip";
pub const WIX_SHA256: &str = "2c1888d5d1dba377fc7fa14444cf556963747ff9a0a289a3599cf09da03b9e2e";

const WIX_REQUIRED_FILES: &[&str] = &[
    "candle.exe",
    "candle.exe.config",
    "darice.cub",
    "light.exe",
    "light.exe.config",
    "wconsole.dll",
    "winterop.dll",
    "wix.dll",
    "WixUIExtension.dll",
    "WixUtilExtension.dll",
];

// A v4 UUID that was generated specifically for cargo-packager, to be used as a
// namespace for generating v5 UUIDs from bundle identifier strings.
const UUID_NAMESPACE: [u8; 16] = [
    0xfd, 0x85, 0x95, 0xa8, 0x17, 0xa3, 0x47, 0x4e, 0xa6, 0x16, 0x76, 0x14, 0x8d, 0xfa, 0x0c, 0x7b,
];

#[derive(Debug, Deserialize)]
struct LanguageMetadata {
    #[serde(rename = "asciiCode")]
    ascii_code: usize,
    #[serde(rename = "langId")]
    lang_id: usize,
}

/// Generates a GUID.
fn generate_guid(key: &[u8]) -> Uuid {
    let namespace = Uuid::from_bytes(UUID_NAMESPACE);
    Uuid::new_v5(&namespace, key)
}

/// Generates the UUID for the Wix template.
fn generate_package_guid(config: &Config) -> Uuid {
    generate_guid(config.identifier().as_bytes())
}

// WiX requires versions to be numeric only in a `major.minor.patch.build` format
pub fn convert_version(version_str: &str) -> crate::Result<String> {
    let version = semver::Version::parse(version_str)?;
    if version.major > 255 {
        return Err(crate::Error::InvalidAppVersion(
            "major number cannot be greater than 255".into(),
        ));
    }
    if version.minor > 255 {
        return Err(crate::Error::InvalidAppVersion(
            "minor number cannot be greater than 255".into(),
        ));
    }
    if version.patch > 65535 {
        return Err(crate::Error::InvalidAppVersion(
            "patch number cannot be greater than 65535".into(),
        ));
    }

    if !version.build.is_empty() {
        let build = version.build.parse::<u64>();
        if build.map(|b| b <= 65535).unwrap_or_default() {
            return Ok(format!(
                "{}.{}.{}.{}",
                version.major, version.minor, version.patch, version.build
            ));
        } else {
            return Err(crate::Error::NonNumericBuildMetadata(Some(
                "and cannot be greater than 65535 for msi target".into(),
            )));
        }
    }

    if !version.pre.is_empty() {
        let pre = version.pre.parse::<u64>();
        if pre.is_ok() && pre.unwrap() <= 65535 {
            return Ok(format!(
                "{}.{}.{}.{}",
                version.major, version.minor, version.patch, version.pre
            ));
        } else {
            return Err(crate::Error::NonNumericBuildMetadata(Some(
                "and cannot be greater than 65535 for msi target".into(),
            )));
        }
    }

    Ok(version_str.to_string())
}

/// A binary to bundle with WIX.
/// External binaries or additional project binaries are represented with this data structure.
/// This data structure is needed because WIX requires each path to have its own `id` and `guid`.
#[derive(Serialize)]
struct Binary {
    /// the GUID to use on the WIX XML.
    guid: String,
    /// the id to use on the WIX XML.
    id: String,
    /// the binary path.
    path: String,
}

/// Generates the data required for the external binaries.
#[tracing::instrument(level = "trace")]
fn generate_binaries_data(config: &Config) -> crate::Result<Vec<Binary>> {
    let mut binaries = Vec::new();
    let cwd = std::env::current_dir()?;
    let tmp_dir = std::env::temp_dir();
    let regex = Regex::new(r"[^\w\d\.]")?;

    if let Some(external_binaries) = &config.external_binaries {
        for src in external_binaries {
            let src = src.with_extension("exe");
            let bin_path = dunce::canonicalize(cwd.join(src))?;
            let dest_filename = bin_path
                .file_name()
                .ok_or_else(|| crate::Error::FailedToExtractFilename(bin_path.clone()))?
                .to_string_lossy()
                .replace(&format!("-{}", config.target_triple()), "");
            let dest = tmp_dir.join(&dest_filename);
            std::fs::copy(bin_path, &dest)?;

            binaries.push(Binary {
                guid: Uuid::new_v4().to_string(),
                path: dest.into_os_string().into_string().unwrap_or_default(),
                id: regex
                    .replace_all(&dest_filename.replace('-', "_"), "")
                    .to_string(),
            });
        }
    }

    for bin in &config.binaries {
        if !bin.main {
            binaries.push(Binary {
                guid: Uuid::new_v4().to_string(),
                path: config
                    .binary_path(bin)
                    .with_extension("exe")
                    .into_os_string()
                    .into_string()
                    .unwrap_or_default(),
                id: regex
                    .replace_all(
                        &bin.path
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .replace('-', "_"),
                        "",
                    )
                    .to_string(),
            })
        }
    }

    Ok(binaries)
}

/// A Resource file to bundle with WIX.
/// This data structure is needed because WIX requires each path to have its own `id` and `guid`.
#[derive(Serialize, Clone)]
struct ResourceFile {
    /// the GUID to use on the WIX XML.
    guid: String,
    /// the id to use on the WIX XML.
    id: String,
    /// the file path.
    path: PathBuf,
}

/// A resource directory to bundle with WIX.
/// This data structure is needed because WIX requires each path to have its own `id` and `guid`.
#[derive(Serialize)]
struct ResourceDirectory {
    /// the directory path.
    path: String,
    /// the directory name of the described resource.
    name: String,
    /// the files of the described resource directory.
    files: Vec<ResourceFile>,
    /// the directories that are children of the described resource directory.
    directories: Vec<ResourceDirectory>,
}

impl ResourceDirectory {
    /// Adds a file to this directory descriptor.
    fn add_file(&mut self, file: ResourceFile) {
        self.files.push(file);
    }

    /// Generates the wix XML string to bundle this directory resources recursively
    fn get_wix_data(self) -> crate::Result<(String, Vec<String>)> {
        let mut files = String::from("");
        let mut file_ids = Vec::new();
        for file in self.files {
            file_ids.push(file.id.clone());
            files.push_str(
          format!(
            r#"<Component Id="{id}" Guid="{guid}" Win64="$(var.Win64)" KeyPath="yes"><File Id="PathFile_{id}" Source="{path}" /></Component>"#,
            id = file.id,
            guid = file.guid,
            path = file.path.display()
          ).as_str()
        );
        }
        let mut directories = String::from("");
        for directory in self.directories {
            let (wix_string, ids) = directory.get_wix_data()?;
            for id in ids {
                file_ids.push(id)
            }
            directories.push_str(wix_string.as_str());
        }
        let wix_string = if self.name.is_empty() {
            format!("{}{}", files, directories)
        } else {
            format!(
                r#"<Directory Id="I{id}" Name="{name}">{files}{directories}</Directory>"#,
                id = Uuid::new_v4().as_simple(),
                name = self.name,
                files = files,
                directories = directories,
            )
        };

        Ok((wix_string, file_ids))
    }
}

/// Mapper between a resource directory name and its ResourceDirectory descriptor.
type ResourceMap = BTreeMap<String, ResourceDirectory>;

/// Generates the data required for the resource on wix
#[tracing::instrument(level = "trace")]
fn generate_resource_data(config: &Config) -> crate::Result<ResourceMap> {
    let mut resources_map = ResourceMap::new();
    for resource in config.resources()? {
        let resource_entry = ResourceFile {
            id: format!("I{}", Uuid::new_v4().as_simple()),
            guid: Uuid::new_v4().to_string(),
            path: resource.src,
        };

        // split the resource path directories
        let components_count = resource.target.components().count();
        let directories = resource
            .target
            .components()
            .take(components_count - 1) // the last component is the file
            .collect::<Vec<_>>();

        // transform the directory structure to a chained vec structure
        let first_directory = directories
            .first()
            .map(|d| d.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_else(String::new);

        if !resources_map.contains_key(&first_directory) {
            resources_map.insert(
                first_directory.clone(),
                ResourceDirectory {
                    path: first_directory.clone(),
                    name: first_directory.clone(),
                    directories: vec![],
                    files: vec![],
                },
            );
        }

        let mut directory_entry = resources_map.get_mut(&first_directory).unwrap();

        let mut path = String::new();
        // the first component is already parsed on `first_directory` so we skip(1)
        for directory in directories.into_iter().skip(1) {
            let directory_name = directory
                .as_os_str()
                .to_os_string()
                .into_string()
                .unwrap_or_default();
            path.push_str(directory_name.as_str());
            path.push(std::path::MAIN_SEPARATOR);

            let index = directory_entry
                .directories
                .iter()
                .position(|f| f.path == path);
            match index {
                Some(i) => directory_entry = directory_entry.directories.get_mut(i).unwrap(),
                None => {
                    directory_entry.directories.push(ResourceDirectory {
                        path: path.clone(),
                        name: directory_name,
                        directories: vec![],
                        files: vec![],
                    });
                    directory_entry = directory_entry.directories.iter_mut().last().unwrap();
                }
            }
        }
        directory_entry.add_file(resource_entry);
    }

    Ok(resources_map)
}

#[derive(Serialize)]
struct MergeModule<'a> {
    name: &'a str,
    path: &'a PathBuf,
}

fn clear_env_for_wix(cmd: &mut Command) {
    cmd.env_clear();
    let required_vars: Vec<std::ffi::OsString> =
        vec!["SYSTEMROOT".into(), "TMP".into(), "TEMP".into()];
    for (k, v) in std::env::vars_os() {
        let k = k.to_ascii_uppercase();
        if required_vars.contains(&k) || k.to_string_lossy().starts_with("CARGO_PACKAGER") {
            cmd.env(k, v);
        }
    }
}

/// Runs the Candle.exe executable for Wix. Candle parses the wxs file and generates the code for building the installer.
fn run_candle(
    config: &Config,
    wix_path: &Path,
    intermediates_path: &Path,
    arch: &str,
    wxs_file_path: PathBuf,
    extensions: Vec<PathBuf>,
) -> crate::Result<()> {
    let main_binary = config.main_binary()?;
    let mut args = vec![
        "-arch".to_string(),
        arch.to_string(),
        wxs_file_path.to_string_lossy().to_string(),
        format!(
            "-dSourceDir={}",
            util::display_path(config.binary_path(main_binary))
        ),
    ];

    if config.wix().map(|w| w.fips_compliant).unwrap_or_default() {
        args.push("-fips".into());
    }

    let candle_exe = wix_path.join("candle.exe");

    tracing::info!("Running candle for {:?}", wxs_file_path);
    let mut cmd = Command::new(candle_exe);
    for ext in extensions {
        cmd.arg("-ext");
        cmd.arg(ext);
    }

    clear_env_for_wix(&mut cmd);

    if let Some(level) = config.log_level {
        if level >= LogLevel::Debug {
            cmd.arg("-v");
        }
    }

    cmd.args(&args)
        .current_dir(intermediates_path)
        .output_ok()
        .map_err(|e| crate::Error::WixFailed("candle.exe".into(), e))?;

    Ok(())
}

/// Runs the Light.exe file. Light takes the generated code from Candle and produces an MSI Installer.
fn run_light(
    config: &Config,
    wix_path: &Path,
    intermediates_path: &Path,
    arguments: Vec<String>,
    extensions: &Vec<PathBuf>,
    output_path: &Path,
) -> crate::Result<()> {
    let light_exe = wix_path.join("light.exe");

    let mut args: Vec<String> = vec!["-o".to_string(), util::display_path(output_path)];

    args.extend(arguments);

    let mut cmd = Command::new(light_exe);
    for ext in extensions {
        cmd.arg("-ext");
        cmd.arg(ext);
    }

    clear_env_for_wix(&mut cmd);

    if let Some(level) = config.log_level {
        if level >= LogLevel::Debug {
            cmd.arg("-v");
        }
    }

    cmd.args(&args)
        .current_dir(intermediates_path)
        .output_ok()
        .map_err(|e| crate::Error::WixFailed("light.exe".into(), e))?;

    Ok(())
}

#[tracing::instrument(level = "trace")]
fn get_and_extract_wix(path: &Path) -> crate::Result<()> {
    let data = download_and_verify(
        "wix311-binaries.zip",
        WIX_URL,
        WIX_SHA256,
        HashAlgorithm::Sha256,
    )?;
    tracing::info!("extracting WIX");
    extract_zip(&data, path)
}

#[tracing::instrument(level = "trace")]
fn build_wix_app_installer(ctx: &Context, wix_path: &Path) -> crate::Result<Vec<PathBuf>> {
    let Context {
        config,
        intermediates_path,
        ..
    } = ctx;

    let arch = match config.target_arch()? {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        target => return Err(crate::Error::UnsupportedArch("wix".into(), target.into())),
    };

    let main_binary = config.main_binary()?;
    let main_binary_name = config.main_binary_name()?;
    let main_binary_path = config.binary_path(main_binary).with_extension("exe");

    tracing::debug!("Codesigning {}", main_binary_path.display());
    codesign::try_sign(&main_binary_path, config)?;

    let intermediates_path = intermediates_path.join("wix").join(arch);
    util::create_clean_dir(&intermediates_path)?;

    let mut data = BTreeMap::new();

    data.insert("product_name", to_json(&config.product_name));
    data.insert("version", to_json(convert_version(&config.version)?));
    let identifier = config.identifier();
    let manufacturer = config.publisher();
    data.insert("identifier", to_json(identifier));
    data.insert("manufacturer", to_json(manufacturer));
    let upgrade_code = Uuid::new_v5(
        &Uuid::NAMESPACE_DNS,
        format!("{}.app.x64", main_binary_name).as_bytes(),
    )
    .to_string();

    data.insert("upgrade_code", to_json(upgrade_code.as_str()));
    data.insert(
        "allow_downgrades",
        to_json(config.windows().map(|w| w.allow_downgrades).unwrap_or(true)),
    );

    let path_guid = generate_package_guid(config).to_string();
    data.insert("path_component_guid", to_json(path_guid.as_str()));

    let shortcut_guid = generate_package_guid(config).to_string();
    data.insert("shortcut_guid", to_json(shortcut_guid.as_str()));

    let binaries = generate_binaries_data(config)?;
    data.insert("binaries", to_json(binaries));

    let resources = generate_resource_data(config)?;
    let mut resources_wix_string = String::from("");
    let mut files_ids = Vec::new();
    for (_, dir) in resources {
        let (wix_string, ids) = dir.get_wix_data()?;
        resources_wix_string.push_str(wix_string.as_str());
        for id in ids {
            files_ids.push(id);
        }
    }
    data.insert("resources", to_json(resources_wix_string));
    data.insert("resource_file_ids", to_json(files_ids));

    data.insert("app_exe_source", to_json(&main_binary_path));

    // copy icon from `settings.windows().icon_path` folder to resource folder near msi
    if let Some(icon) = config.find_ico() {
        let icon_path = dunce::canonicalize(icon)?;
        data.insert("icon_path", to_json(icon_path));
    }

    if let Some(license) = &config.license_file {
        if license.ends_with(".rtf") {
            data.insert("license", to_json(license));
        } else {
            let license_contents = std::fs::read_to_string(license)?;
            let license_rtf = format!(
                r#"{{\rtf1\ansi\ansicpg1252\deff0\nouicompat\deflang1033{{\fonttbl{{\f0\fnil\fcharset0 Calibri;}}}}
{{\*\generator Riched20 10.0.18362}}\viewkind4\uc1
\pard\sa200\sl276\slmult1\f0\fs22\lang9 {}\par
}}
 "#,
                license_contents.replace('\n', "\\par ")
            );
            let rtf_output_path = intermediates_path.join("LICENSE.rtf");
            tracing::debug!("Writing {}", util::display_path(&rtf_output_path));
            std::fs::write(&rtf_output_path, license_rtf)?;
            data.insert("license", to_json(rtf_output_path));
        }
    }

    let mut fragment_paths = Vec::new();
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    let mut custom_template_path = None;
    if let Some(wix) = config.wix() {
        data.insert("custom_action_refs", to_json(&wix.custom_action_refs));
        data.insert("component_group_refs", to_json(&wix.component_group_refs));
        data.insert("component_refs", to_json(&wix.component_refs));
        data.insert("feature_group_refs", to_json(&wix.feature_group_refs));
        data.insert("feature_refs", to_json(&wix.feature_refs));
        data.insert("merge_refs", to_json(&wix.merge_refs));
        custom_template_path = wix.template.clone();

        fragment_paths = wix.fragment_paths.clone().unwrap_or_default();
        if let Some(ref inline_fragments) = wix.fragments {
            tracing::debug!(
                "Writing inline fragments to {}",
                util::display_path(&intermediates_path)
            );
            for (idx, fragment) in inline_fragments.iter().enumerate() {
                let path = intermediates_path.join(format!("inline_fragment{idx}.wxs"));
                std::fs::write(&path, fragment)?;
                fragment_paths.push(path);
            }
        }

        if let Some(banner_path) = &wix.banner_path {
            data.insert("banner_path", to_json(dunce::canonicalize(banner_path)?));
        }

        if let Some(dialog_image_path) = &wix.dialog_image_path {
            data.insert(
                "dialog_image_path",
                to_json(dunce::canonicalize(dialog_image_path)?),
            );
        }

        if let Some(merge_modules) = &wix.merge_modules {
            let merge_modules = merge_modules
                .iter()
                .map(|path| MergeModule {
                    name: path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or_default(),
                    path,
                })
                .collect::<Vec<_>>();
            data.insert("merge_modules", to_json(merge_modules));
        }
    }

    if let Some(file_associations) = &config.file_associations {
        data.insert("file_associations", to_json(file_associations));
    }

    if let Some(path) = custom_template_path {
        handlebars
            .register_template_string("main.wxs", std::fs::read_to_string(path)?)
            .map_err(Box::new)?;
    } else {
        handlebars
            .register_template_string("main.wxs", include_str!("./main.wxs"))
            .map_err(Box::new)?;
    }

    let main_wxs_path = intermediates_path.join("main.wxs");
    tracing::debug!("Writing {}", util::display_path(&main_wxs_path));
    std::fs::write(&main_wxs_path, handlebars.render("main.wxs", &data)?)?;

    let mut candle_inputs = vec![(main_wxs_path, Vec::new())];

    let current_dir = std::env::current_dir()?;
    let extension_regex = Regex::new("\"http://schemas.microsoft.com/wix/(\\w+)\"")?;
    for fragment_path in fragment_paths {
        let fragment_path = current_dir.join(fragment_path);
        let fragment = std::fs::read_to_string(&fragment_path)?;
        let mut extensions = Vec::new();
        for cap in extension_regex.captures_iter(&fragment) {
            extensions.push(wix_path.join(format!("Wix{}.dll", &cap[1])));
        }
        candle_inputs.push((fragment_path, extensions));
    }

    let mut fragment_extensions = HashSet::new();
    //Default extensions
    fragment_extensions.insert(wix_path.join("WixUIExtension.dll"));
    fragment_extensions.insert(wix_path.join("WixUtilExtension.dll"));

    for (path, extensions) in candle_inputs {
        for ext in &extensions {
            fragment_extensions.insert(ext.clone());
        }
        run_candle(
            config,
            wix_path,
            &intermediates_path,
            arch,
            path,
            extensions,
        )?;
    }

    let mut output_paths = Vec::new();

    let language_map: HashMap<String, LanguageMetadata> =
        serde_json::from_str(include_str!("./languages.json"))?;
    let configured_languages = config
        .wix()
        .and_then(|w| w.languages.clone())
        .unwrap_or_else(|| vec![WixLanguage::default()]);
    for language in configured_languages {
        let (language, locale_path) = match language {
            WixLanguage::Identifier(identifier) => (identifier, None),
            WixLanguage::Custom { identifier, path } => (identifier, path),
        };

        let language_metadata = language_map.get(&language).ok_or_else(|| {
            crate::Error::UnsupportedWixLanguage(
                language.clone(),
                language_map
                    .keys()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(", "),
            )
        })?;

        let locale_contents = match locale_path {
            Some(p) => std::fs::read_to_string(p)?,
            None => format!(
                r#"<WixLocalization Culture="{}" xmlns="http://schemas.microsoft.com/wix/2006/localization"></WixLocalization>"#,
                language.to_lowercase(),
            ),
        };

        let locale_strings = include_str!("./default-locale-strings.xml")
            .replace("__language__", &language_metadata.lang_id.to_string())
            .replace("__codepage__", &language_metadata.ascii_code.to_string())
            .replace("__productName__", &config.product_name);

        let mut unset_locale_strings = String::new();
        let prefix_len = "<String ".len();
        for locale_string in locale_strings.split('\n').filter(|s| !s.is_empty()) {
            // strip `<String ` prefix and `>{value}</String` suffix.
            let id = locale_string
                .chars()
                .skip(prefix_len)
                .take(locale_string.find('>').unwrap() - prefix_len)
                .collect::<String>();
            if !locale_contents.contains(&id) {
                unset_locale_strings.push_str(locale_string);
            }
        }

        let locale_contents = locale_contents.replace(
            "</WixLocalization>",
            &format!("{}</WixLocalization>", unset_locale_strings),
        );
        let locale_path = intermediates_path.join("locale.wxl");
        {
            tracing::debug!("Writing {}", util::display_path(&locale_path));
            let mut fileout = File::create(&locale_path)?;
            fileout.write_all(locale_contents.as_bytes())?;
        }

        let arguments = vec![
            format!(
                "-cultures:{}",
                if language == "en-US" {
                    language.to_lowercase()
                } else {
                    format!("{};en-US", language.to_lowercase())
                }
            ),
            "-loc".into(),
            util::display_path(&locale_path),
            "*.wixobj".into(),
        ];
        let msi_output_path = intermediates_path.join("output.msi");
        let msi_path = config.out_dir().join(format!(
            "{}_{}_{}_{}.msi",
            main_binary_name, config.version, arch, language
        ));
        std::fs::create_dir_all(
            msi_path
                .parent()
                .ok_or_else(|| crate::Error::ParentDirNotFound(msi_path.clone()))?,
        )?;

        tracing::info!(
            "Running light.exe to produce {}",
            util::display_path(&msi_path)
        );

        run_light(
            config,
            wix_path,
            &intermediates_path,
            arguments,
            &(fragment_extensions.clone().into_iter().collect()),
            &msi_output_path,
        )?;
        std::fs::rename(&msi_output_path, &msi_path)?;
        tracing::debug!("Codesigning {}", msi_path.display());
        codesign::try_sign(&msi_path, config)?;
        output_paths.push(msi_path);
    }

    Ok(output_paths)
}

#[tracing::instrument(level = "trace")]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let wix_path = ctx.tools_path.join("WixTools");
    if !wix_path.exists() {
        get_and_extract_wix(&wix_path)?;
    } else if WIX_REQUIRED_FILES
        .iter()
        .any(|p| !wix_path.join(p).exists())
    {
        tracing::warn!("WixTools directory is missing some files. Recreating it.");
        std::fs::remove_dir_all(&wix_path)?;
        get_and_extract_wix(&wix_path)?;
    }

    build_wix_app_installer(ctx, &wix_path)
}
