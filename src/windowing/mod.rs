#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) mod windows_gdi;

#[cfg(target_os = "windows")]
pub(crate) mod gdi;

#[cfg(target_os = "windows")]
pub use windows::create;

pub enum WindowEvent {
    Closed,
}

pub trait Window {
    fn run_on_ui_thread(&self, runnable: impl FnOnce() -> () + 'static + Send);
}
