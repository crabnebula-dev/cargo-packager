use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use cargo_packager_config::LogLevel;
use handlebars::{to_json, Handlebars};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    config::{Config, ConfigExt, ConfigExtInternal},
    shell::CommandExt,
    sign,
    util::{display_path, download_and_verify, extract_zip, HashAlgorithm},
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
fn generate_binaries_data(config: &Config) -> crate::Result<Vec<Binary>> {
    let mut binaries = Vec::new();
    let cwd = std::env::current_dir()?;
    let tmp_dir = std::env::temp_dir();
    let regex = Regex::new(r"[^\w\d\.]")?;

    if let Some(external_binaries) = &config.external_binaries {
        for src in external_binaries {
            let binary_path = cwd.join(src);
            let dest_filename = PathBuf::from(src)
                .file_name()
                .expect("failed to extract external binary filename")
                .to_string_lossy()
                .replace(&format!("-{}", config.target_triple()), "");
            let dest = tmp_dir.join(&dest_filename);
            std::fs::copy(binary_path, &dest)?;

            binaries.push(Binary {
                guid: Uuid::new_v4().to_string(),
                path: dest
                    .into_os_string()
                    .into_string()
                    .expect("failed to read external binary path"),
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
                    .into_os_string()
                    .into_string()
                    .expect("failed to read binary path"),
                id: regex
                    .replace_all(&bin.filename.replace('-', "_"), "")
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

        let mut directory_entry = resources_map
            .get_mut(&first_directory)
            .expect("Unable to handle resources");

        let mut path = String::new();
        // the first component is already parsed on `first_directory` so we skip(1)
        for directory in directories.into_iter().skip(1) {
            let directory_name = directory
                .as_os_str()
                .to_os_string()
                .into_string()
                .expect("failed to read resource folder name");
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

/// Copies the icon to the binary path, under the `resources` folder,
/// and returns the path to the file.
fn copy_icon(config: &Config, filename: &str, path: &Path) -> crate::Result<PathBuf> {
    let resource_dir = config.out_dir().join("resources");
    std::fs::create_dir_all(&resource_dir)?;
    let icon_target_path = resource_dir.join(filename);
    let icon_path = std::env::current_dir()?.join(path);
    std::fs::copy(icon_path, &icon_target_path)?;
    Ok(icon_target_path)
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
    arch: &str,
    wix_toolset_path: &Path,
    cwd: &Path,
    wxs_file_path: PathBuf,
    extensions: Vec<PathBuf>,
    log_level: LogLevel,
) -> crate::Result<()> {
    let main_binary = config.main_binary()?;
    let mut args = vec![
        "-arch".to_string(),
        arch.to_string(),
        wxs_file_path.to_string_lossy().to_string(),
        format!(
            "-dSourceDir={}",
            display_path(config.binary_path(main_binary))
        ),
    ];

    if config.wix().map(|w| w.fips_compliant).unwrap_or_default() {
        args.push("-fips".into());
    }

    let candle_exe = wix_toolset_path.join("candle.exe");

    log::info!(action = "Running"; "candle for {:?}", wxs_file_path);
    let mut cmd = Command::new(candle_exe);
    for ext in extensions {
        cmd.arg("-ext");
        cmd.arg(ext);
    }
    clear_env_for_wix(&mut cmd);
    if log_level >= LogLevel::Debug {
        cmd.arg("-v");
    }

    cmd.args(&args)
        .current_dir(cwd)
        .output_ok()
        .map_err(|e| crate::Error::WixFailed("candle.exe".into(), e.to_string()))?;

    Ok(())
}

/// Runs the Light.exe file. Light takes the generated code from Candle and produces an MSI Installer.
fn run_light(
    wix_toolset_path: &Path,
    build_path: &Path,
    arguments: Vec<String>,
    extensions: &Vec<PathBuf>,
    output_path: &Path,
    log_level: LogLevel,
) -> crate::Result<()> {
    let light_exe = wix_toolset_path.join("light.exe");

    let mut args: Vec<String> = vec!["-o".to_string(), display_path(output_path)];

    args.extend(arguments);

    let mut cmd = Command::new(light_exe);
    for ext in extensions {
        cmd.arg("-ext");
        cmd.arg(ext);
    }
    clear_env_for_wix(&mut cmd);
    if log_level >= LogLevel::Debug {
        cmd.arg("-v");
    }
    cmd.args(&args)
        .current_dir(build_path)
        .output_ok()
        .map_err(|e| crate::Error::WixFailed("light.exe".into(), e.to_string()))?;

    Ok(())
}

fn get_and_extract_wix(path: &Path) -> crate::Result<()> {
    let data = download_and_verify(
        "wix311-binaries.zip",
        WIX_URL,
        WIX_SHA256,
        HashAlgorithm::Sha256,
    )?;
    log::info!("extracting WIX");
    extract_zip(&data, path)
}

fn build_wix_app_installer(
    config: &Config,
    wix_toolset_path: &Path,
) -> crate::Result<Vec<PathBuf>> {
    let arch = match config.target_arch()? {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        target => return Err(crate::Error::UnsupportedArch("wix".into(), target.into())),
    };

    let main_binary = config.main_binary()?;
    let app_exe_source = config.binary_path(main_binary);

    sign::try_sign(&app_exe_source.with_extension("exe"), config)?;

    let output_path = config.out_dir().join("wix").join(arch);

    if output_path.exists() {
        std::fs::remove_dir_all(&output_path)?;
    }
    std::fs::create_dir_all(&output_path)?;

    let app_version = convert_version(&config.version)?;

    let mut data = BTreeMap::new();

    // TODO: webview2 logic

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
            let rtf_output_path = config.out_dir().join("wix").join("LICENSE.rtf");
            std::fs::write(&rtf_output_path, license_rtf)?;
            data.insert("license", to_json(rtf_output_path));
        }
    }

    data.insert("product_name", to_json(&config.product_name));
    data.insert("version", to_json(&app_version));
    let identifier = config.identifier();
    let manufacturer = config.publisher();
    data.insert("identifier", to_json(identifier));
    data.insert("manufacturer", to_json(manufacturer));
    let upgrade_code = Uuid::new_v5(
        &Uuid::NAMESPACE_DNS,
        format!("{}.app.x64", &main_binary.filename).as_bytes(),
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

    data.insert(
        "app_exe_source",
        to_json(&app_exe_source.with_extension("exe")),
    );

    // copy icon from `settings.windows().icon_path` folder to resource folder near msi
    if let Some(icon) = config.find_ico() {
        let icon_path = copy_icon(config, "icon.ico", &icon)?;
        data.insert("icon_path", to_json(icon_path));
    }

    let mut fragment_paths = Vec::new();
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    let mut custom_template_path = None;

    if let Some(wix) = config.wix() {
        data.insert("component_group_refs", to_json(&wix.component_group_refs));
        data.insert("component_refs", to_json(&wix.component_refs));
        data.insert("feature_group_refs", to_json(&wix.feature_group_refs));
        data.insert("feature_refs", to_json(&wix.feature_refs));
        data.insert("merge_refs", to_json(&wix.merge_refs));
        fragment_paths = wix.fragment_paths.clone().unwrap_or_default();
        custom_template_path = wix.template.clone();

        if let Some(banner_path) = &wix.banner_path {
            let filename = banner_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            data.insert(
                "banner_path",
                to_json(copy_icon(config, &filename, banner_path)?),
            );
        }

        if let Some(dialog_image_path) = &wix.dialog_image_path {
            let filename = dialog_image_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            data.insert(
                "dialog_image_path",
                to_json(copy_icon(config, &filename, dialog_image_path)?),
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
            .map_err(|e| e.to_string())
            .expect("Failed to setup custom handlebar template");
    } else {
        handlebars
            .register_template_string("main.wxs", include_str!("./main.wxs"))
            .map_err(|e| e.to_string())
            .expect("Failed to setup handlebar template");
    }

    let main_wxs_path = output_path.join("main.wxs");
    std::fs::write(&main_wxs_path, handlebars.render("main.wxs", &data)?)?;

    let mut candle_inputs = vec![(main_wxs_path, Vec::new())];

    let current_dir = std::env::current_dir()?;
    let extension_regex = Regex::new("\"http://schemas.microsoft.com/wix/(\\w+)\"")?;
    for fragment_path in fragment_paths {
        let fragment_path = current_dir.join(fragment_path);
        let fragment = std::fs::read_to_string(&fragment_path)?;
        let mut extensions = Vec::new();
        for cap in extension_regex.captures_iter(&fragment) {
            extensions.push(wix_toolset_path.join(format!("Wix{}.dll", &cap[1])));
        }
        candle_inputs.push((fragment_path, extensions));
    }

    let mut fragment_extensions = HashSet::new();
    //Default extensions
    fragment_extensions.insert(wix_toolset_path.join("WixUIExtension.dll"));
    fragment_extensions.insert(wix_toolset_path.join("WixUtilExtension.dll"));

    for (path, extensions) in candle_inputs {
        for ext in &extensions {
            fragment_extensions.insert(ext.clone());
        }
        run_candle(
            config,
            arch,
            wix_toolset_path,
            &output_path,
            path,
            extensions,
            config.log_level.unwrap_or_default(),
        )?;
    }

    let mut output_paths = Vec::new();

    let language_map: HashMap<String, LanguageMetadata> =
        serde_json::from_str(include_str!("./languages.json")).unwrap();
    let configured_languages = config
        .wix()
        .map(|w| w.languages.clone())
        .unwrap_or_default();
    for (language, language_config) in configured_languages.0 {
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

        let locale_contents = match language_config.locale_path {
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
        let locale_path = output_path.join("locale.wxl");
        {
            let mut fileout = File::create(&locale_path).expect("Failed to create locale file");
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
            display_path(&locale_path),
            "*.wixobj".into(),
        ];
        let msi_output_path = output_path.join("output.msi");
        let msi_path = config.out_dir().join(format!(
            "{}_{}_{}_{}.msi",
            main_binary.filename, app_version, arch, language
        ));
        std::fs::create_dir_all(msi_path.parent().unwrap())?;

        log::info!(action = "Running"; "light.exe to produce {}", display_path(&msi_path));

        run_light(
            wix_toolset_path,
            &output_path,
            arguments,
            &(fragment_extensions.clone().into_iter().collect()),
            &msi_output_path,
            config.log_level.unwrap_or_default(),
        )?;
        std::fs::rename(&msi_output_path, &msi_path)?;
        sign::try_sign(&msi_path, config)?;
        output_paths.push(msi_path);
    }

    Ok(output_paths)
}

pub fn package(config: &Config) -> crate::Result<Vec<PathBuf>> {
    let wix_path = dirs::cache_dir().unwrap().join("cargo-packager/WixTools");
    if !wix_path.exists() {
        get_and_extract_wix(&wix_path)?;
    } else if WIX_REQUIRED_FILES
        .iter()
        .any(|p| !wix_path.join(p).exists())
    {
        log::warn!("WixTools directory is missing some files. Recreating it.");
        std::fs::remove_dir_all(&wix_path)?;
        get_and_extract_wix(&wix_path)?;
    }

    build_wix_app_installer(config, &wix_path)
}
