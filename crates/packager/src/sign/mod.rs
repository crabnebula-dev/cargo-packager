#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod imp;

#[cfg(windows)]
#[path = "windows.rs"]
mod imp;

#[cfg(any(windows, target_os = "macos"))]
pub use imp::*;
