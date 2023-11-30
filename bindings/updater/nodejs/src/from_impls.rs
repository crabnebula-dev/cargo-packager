use crate::{Options, Update, UpdateFormat, UpdaterWindowsOptions, WindowsUpdateInstallMode};

impl From<WindowsUpdateInstallMode> for cargo_packager_updater::WindowsUpdateInstallMode {
    fn from(value: WindowsUpdateInstallMode) -> Self {
        match value {
            WindowsUpdateInstallMode::BasicUi => Self::BasicUi,
            WindowsUpdateInstallMode::Quiet => Self::Quiet,
            WindowsUpdateInstallMode::Passive => Self::Passive,
        }
    }
}

impl From<cargo_packager_updater::WindowsUpdateInstallMode> for WindowsUpdateInstallMode {
    fn from(value: cargo_packager_updater::WindowsUpdateInstallMode) -> Self {
        match value {
            cargo_packager_updater::WindowsUpdateInstallMode::BasicUi => Self::BasicUi,
            cargo_packager_updater::WindowsUpdateInstallMode::Quiet => Self::Quiet,
            cargo_packager_updater::WindowsUpdateInstallMode::Passive => Self::Passive,
        }
    }
}

impl From<cargo_packager_updater::UpdaterWindowsConfig> for UpdaterWindowsOptions {
    fn from(value: cargo_packager_updater::UpdaterWindowsConfig) -> Self {
        Self {
            installer_args: Some(value.installer_args),
            install_mode: Some(value.install_mode.into()),
        }
    }
}
impl From<UpdaterWindowsOptions> for cargo_packager_updater::UpdaterWindowsConfig {
    fn from(value: UpdaterWindowsOptions) -> Self {
        Self {
            installer_args: value.installer_args.unwrap_or_default(),
            install_mode: value.install_mode.map(Into::into).unwrap_or_default(),
        }
    }
}

impl From<Options> for cargo_packager_updater::Config {
    fn from(value: Options) -> Self {
        Self {
            endpoints: value
                .endpoints
                .into_iter()
                .filter_map(|e| e.parse().ok())
                .collect(),
            pubkey: value.pubkey,
            windows: value.windows.map(Into::into).unwrap_or_default(),
        }
    }
}

impl From<cargo_packager_updater::UpdateFormat> for UpdateFormat {
    fn from(value: cargo_packager_updater::UpdateFormat) -> Self {
        match value {
            cargo_packager_updater::UpdateFormat::Nsis => Self::Nsis,
            cargo_packager_updater::UpdateFormat::Wix => Self::Wix,
            cargo_packager_updater::UpdateFormat::AppImage => Self::AppImage,
            cargo_packager_updater::UpdateFormat::App => Self::App,
        }
    }
}
impl From<UpdateFormat> for cargo_packager_updater::UpdateFormat {
    fn from(value: UpdateFormat) -> Self {
        match value {
            UpdateFormat::Nsis => Self::Nsis,
            UpdateFormat::Wix => Self::Wix,
            UpdateFormat::AppImage => Self::AppImage,
            UpdateFormat::App => Self::App,
        }
    }
}

impl From<cargo_packager_updater::Update> for Update {
    fn from(value: cargo_packager_updater::Update) -> Self {
        Self {
            pubkey: value.config.pubkey,
            body: value.body,
            current_version: value.current_version,
            version: value.version,
            date: value.date.and_then(|d| {
                d.format(&time::format_description::well_known::Rfc3339)
                    .ok()
            }),
            target: value.target,
            extract_path: value.extract_path.to_string_lossy().to_string(),
            download_url: value.download_url.to_string(),
            signature: value.signature,
            timeout: value.timeout.map(|t| t.as_millis() as u32),
            headers: value
                .headers
                .into_iter()
                .map(|(k, v)| {
                    (
                        k.map(|k| k.to_string()).unwrap_or_default(),
                        v.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect(),
            format: value.format.into(),
            windows: value.config.windows.into(),
        }
    }
}
