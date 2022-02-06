use std::string::String;
use winapi::shared::minwindef::{BOOL, FALSE, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::winuser;

struct MainWindowFinder {
    handle: HWND,
    pid: u32,
}

#[cfg(target_os = "windows")]
pub fn find_main_window_title(pid: u32) -> String {
    let mut finder = MainWindowFinder {
        handle: std::ptr::null_mut(),
        pid,
    };

    unsafe { winuser::EnumWindows(Some(enum_windows_callback), &mut finder as *mut _ as isize) };

    let hwnd = finder.handle;
    if hwnd.is_null() {
        return "".into();
    }

    let length = unsafe { winuser::GetWindowTextLengthW(hwnd) };
    if length == 0 {
        return "".into();
    }

    let mut wstr: Vec<u16> = Vec::with_capacity(length as usize + 1);
    unsafe {
        let lpwstr = wstr.as_mut_ptr();
        let ret_length = winuser::GetWindowTextW(hwnd, lpwstr, wstr.capacity() as i32);
        wstr.set_len(ret_length as usize);
    }

    String::from_utf16_lossy(&wstr)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, param: isize) -> BOOL {
    const GW_OWNER: u32 = 4;
    let mut finder: *mut MainWindowFinder = param as _;
    let mut pid: u32 = 0;
    winuser::GetWindowThreadProcessId(hwnd, &mut pid);
    if (*finder).pid == pid
        && winuser::GetWindow(hwnd, GW_OWNER).is_null()
        && winuser::IsWindowVisible(hwnd) != FALSE
    {
        (*finder).handle = hwnd;
        FALSE
    } else {
        TRUE
    }
}
