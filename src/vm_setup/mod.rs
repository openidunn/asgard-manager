#[cfg(target_os = "macos")]
pub mod macos_setup;

#[cfg(target_os = "linux")]
pub mod linux_setup;

#[cfg(target_os = "windows")]
pub mod windows_setup;

pub mod setup_utils;