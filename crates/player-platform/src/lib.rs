//! The home for native windows, media controls, device notifications and sleep policy.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Clone, Copy, Debug)]
pub enum PlatformKind {
    Windows,
    MacOs,
    Linux,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub struct PlatformInfo {
    pub kind: PlatformKind,
    pub name: &'static str,
}

pub fn current() -> PlatformInfo {
    #[cfg(target_os = "windows")]
    return windows::info();
    #[cfg(target_os = "macos")]
    return macos::info();
    #[cfg(target_os = "linux")]
    return linux::info();
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    PlatformInfo {
        kind: PlatformKind::Other,
        name: "Other",
    }
}

pub mod appearance;

pub mod audio_devices;
pub mod directory;
pub mod fonts;
pub mod window_surface;
