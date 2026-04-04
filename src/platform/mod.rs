#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

mod types;

pub use types::PlatformAdapter;

/// Return the platform adapter for the current OS.
pub fn adapter() -> Box<dyn PlatformAdapter> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosAdapter)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxAdapter)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsAdapter)
    }
}
