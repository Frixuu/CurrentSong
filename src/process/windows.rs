use std::string::String;
use windows_sys::{
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::UI::WindowsAndMessaging::*,
};

struct MainWindowData {
    handle: HWND,
    pid: u32,
}

/// Gets an opaque handle to a main window of a process.
/// Note: returned HWND might be zero.
fn find_main_window_by_process(pid: u32) -> HWND {
    // Capture in EnumWindows
    let mut data = MainWindowData { handle: 0, pid };
    unsafe { EnumWindows(Some(enum_windows_callback), &mut data as *mut _ as LPARAM) };
    data.handle
}

/// Fetches title of a process' main window, if it has one.
pub fn find_main_window_title(pid: u32) -> Option<String> {
    // Process might not exist or have no windows
    let hwnd = find_main_window_by_process(pid);
    if hwnd == 0 {
        return None;
    }

    // The "window" might have an empty title. This can be the case for explorer.exe
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length == 0 {
        return Some("".into());
    }

    // Build wide-string (UTF-16) buffer
    let mut wstr = Vec::<u16>::with_capacity(length as usize + 1);
    unsafe {
        let lpwstr = wstr.as_mut_ptr();
        let title_length = GetWindowTextW(hwnd, lpwstr, wstr.capacity() as i32);
        wstr.set_len(title_length as usize);
    }

    String::from_utf16(&wstr).ok()
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, param: LPARAM) -> BOOL {
    // Get process ID of the queried window handle
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);

    let mut data = param as *mut MainWindowData;
    if (*data).pid != pid {
        // Not the process we're looking for
        return true.into();
    }

    let owner = GetWindow(hwnd, GW_OWNER);
    if owner != 0 {
        // Window has an owner, we're searching for a top-level one
        return true.into();
    }

    if IsWindowVisible(hwnd) == 0 {
        // We want a visible window only
        return true.into();
    }

    // Store current window handle in captured context
    (*data).handle = hwnd;
    // Signal EnumWindows we do not want to iterate windows anymore
    false.into()
}
