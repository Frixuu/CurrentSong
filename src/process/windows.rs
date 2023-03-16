use std::string::String;
use windows_sys::{
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::UI::WindowsAndMessaging::*,
};

struct SearchContext {
    pid: u32,
    handle: HWND,
}

/// Gets an opaque handle to a main window of a process.
/// Note: returned HWND might be zero.
fn find_main_window_by_process(pid: u32) -> HWND {
    let mut context = SearchContext { pid, handle: 0 };
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            &mut context as *mut _ as LPARAM,
        )
    };
    context.handle
}

/// Fetches title of a process' main window, if it has one.
pub fn find_main_window_title(pid: u32) -> Option<String> {
    // Process might not exist or have no windows
    let hwnd = find_main_window_by_process(pid);
    if hwnd == 0 {
        return None;
    }

    // The "window" might have an empty title.
    // This can be the case for explorer.exe
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length == 0 {
        return Some("".into());
    }

    let mut title = Vec::with_capacity(length as usize + 1);
    unsafe {
        let length = GetWindowTextW(hwnd, title.as_mut_ptr(), title.capacity() as i32);
        title.set_len(length as usize);
    }

    String::from_utf16(&title).ok()
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, param: LPARAM) -> BOOL {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);

    let mut context = param as *mut SearchContext;
    if (*context).pid != pid {
        // Not the process we're looking for
        return true.into();
    }

    let owner = GetWindow(hwnd, GW_OWNER);
    if owner != 0 {
        // Window has an owner, we're searching for a top-level one
        return true.into();
    }

    if IsWindowVisible(hwnd) == 0 {
        // We want visible windows only
        return true.into();
    }

    (*context).handle = hwnd;
    // Signal EnumWindows we no longer want to iterate
    false.into()
}
