#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) mod windows_gdi;

#[cfg(target_os = "windows")]
pub use windows::create;

pub enum WindowEvent {
    Closed,
}
