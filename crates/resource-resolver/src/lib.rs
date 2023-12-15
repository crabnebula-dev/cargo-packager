use std::ffi::OsString;

use semver::Version;



pub mod platform;
pub mod starting_binary;


/// `tauri::App` package information.
#[derive(Debug, Clone)]
pub struct PackageInfo {
  /// App name
  pub name: String,
  /// App version
  pub version: Version,
  /// The crate authors.
  pub authors: &'static str,
  /// The crate description.
  pub description: &'static str,
  /// The crate name.
  pub crate_name: &'static str,
}

impl PackageInfo {
  /// Returns the application package name.
  /// On macOS and Windows it's the `name` field, and on Linux it's the `name` in `kebab-case`.
  pub fn package_name(&self) -> String {
    #[cfg(target_os = "linux")]
    {
      use heck::ToKebabCase;
      self.name.clone().to_kebab_case()
    }
    #[cfg(not(target_os = "linux"))]
    self.name.clone()
  }
}

/// Information about environment variables.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Env {
  /// The APPIMAGE environment variable.
  #[cfg(target_os = "linux")]
  pub appimage: Option<std::ffi::OsString>,
  /// The APPDIR environment variable.
  #[cfg(target_os = "linux")]
  pub appdir: Option<std::ffi::OsString>,
  /// The command line arguments of the current process.
  pub args_os: Vec<OsString>,
}

#[allow(clippy::derivable_impls)]
impl Default for Env {
  fn default() -> Self {
    let args_os = std::env::args_os().skip(1).collect();
    #[cfg(target_os = "linux")]
    {
      let env = Self {
        #[cfg(target_os = "linux")]
        appimage: std::env::var_os("APPIMAGE"),
        #[cfg(target_os = "linux")]
        appdir: std::env::var_os("APPDIR"),
        args_os,
      };
      if env.appimage.is_some() || env.appdir.is_some() {
        // validate that we're actually running on an AppImage
        // an AppImage is mounted to `/$TEMPDIR/.mount_${appPrefix}${hash}`
        // see https://github.com/AppImage/AppImageKit/blob/1681fd84dbe09c7d9b22e13cdb16ea601aa0ec47/src/runtime.c#L501
        // note that it is safe to use `std::env::current_exe` here since we just loaded an AppImage.
        let is_temp = std::env::current_exe()
          .map(|p| {
            p.display()
              .to_string()
              .starts_with(&format!("{}/.mount_", std::env::temp_dir().display()))
          })
          .unwrap_or(true);

        if !is_temp {
          warn!("`APPDIR` or `APPIMAGE` environment variable found but this application was not detected as an AppImage; this might be a security issue.");
        }
      }
      env
    }
    #[cfg(not(target_os = "linux"))]
    {
      Self { args_os }
    }
  }
}

/// The result type of `tauri-utils`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type of `tauri-utils`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
  /// Target triple architecture error
  #[error("Unable to determine target-architecture")]
  Architecture,
  /// Target triple OS error
  #[error("Unable to determine target-os")]
  Os,
  /// Target triple environment error
  #[error("Unable to determine target-environment")]
  Environment,
  /// Tried to get resource on an unsupported platform
  #[error("Unsupported platform for reading resources")]
  UnsupportedPlatform,
  /// Get parent process error
  #[error("Could not get parent process")]
  ParentProcess,
  /// Get parent process PID error
  #[error("Could not get parent PID")]
  ParentPid,
  /// Get child process error
  #[error("Could not get child process")]
  ChildProcess,
  /// IO error
  #[error("{0}")]
  Io(#[from] std::io::Error),
  /// Invalid pattern.
  #[error("invalid pattern `{0}`. Expected either `brownfield` or `isolation`.")]
  InvalidPattern(String),
}