use std::{ffi::OsStr, os::windows::prelude::OsStrExt};
use thiserror::Error;
use windows_sys::{
    core::PCWSTR,
    Win32::Foundation::{HWND, LPARAM},
    Win32::{
        Foundation::{HINSTANCE, LRESULT, WPARAM},
        Graphics::Gdi::HBRUSH,
        UI::WindowsAndMessaging::*,
    },
};

struct WindowClass {
    name: Vec<u16>,
    _atom: u16,
}

#[derive(Error, Debug)]
enum ClassRegisterError {
    #[error("class name was {0} chars long, max is 256")]
    ClassNameTooLong(usize),
    #[error("RegisterClassW returned zero")]
    WindowsError,
}

impl WindowClass {
    /// Attempts to register a window class.
    fn try_register(name: &str, mut class: WNDCLASSW) -> Result<WindowClass, ClassRegisterError> {
        let class_name = prepare_string(name);
        if class_name.len() > 256 {
            return Err(ClassRegisterError::ClassNameTooLong(class_name.len()));
        }
        class.lpszClassName = class_name.as_ptr();

        let result = unsafe { RegisterClassW(&class) };
        match result {
            0 => Err(ClassRegisterError::WindowsError),
            _ => Ok(WindowClass {
                name: class_name,
                _atom: result,
            }),
        }
    }

    fn name_ptr(&self) -> *const u16 {
        self.name.as_ptr()
    }
}

struct Window {
    hwnd: HWND,
}

impl Window {
    fn try_create(class: &WindowClass, title: &str) -> Result<Window, ()> {
        unsafe {
            let (window_width, window_height) = (400, 300);
            let (screen_width, screen_height) =
                (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));

            let title_wide = prepare_string(title);
            let window_name: PCWSTR = title_wide.as_ptr();
            let parent: HWND = 0 as HWND;
            let hwnd = CreateWindowExW(
                0,
                class.name_ptr(),
                window_name,
                WS_OVERLAPPED | WS_MINIMIZEBOX | WS_SYSMENU | WS_CAPTION | WS_BORDER,
                (screen_width - window_width) / 2,
                (screen_height - window_height) / 2,
                window_width,
                window_height,
                parent,
                0 as HMENU,
                0 as HINSTANCE,
                std::ptr::null_mut(),
            );

            match hwnd {
                0 => Err(()),
                _ => Ok(Window { hwnd }),
            }
        }
    }

    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Activates the window and displays it in its current size and position.
    fn show(&self) {
        unsafe { ShowWindow(self.hwnd(), SW_SHOW) };
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => PostQuitMessage(0),
        _ => {}
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn prepare_string(text: &str) -> Vec<u16> {
    let mut s: Vec<u16> = OsStr::new(text).encode_wide().collect();
    s.push(0);
    s
}

pub fn create() {
    let class = WindowClass::try_register(
        "CurrentSongWindowClass",
        WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: 0 as HINSTANCE,
            hIcon: 0 as HICON,
            hCursor: 0 as HICON,
            hbrBackground: COLOR_BTNSHADOW as HBRUSH,
            lpszMenuName: std::ptr::null_mut(),
            lpszClassName: std::ptr::null_mut(),
        },
    )
    .unwrap();

    let window = Window::try_create(&class, "Current Song").unwrap();
    window.show();

    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        // process messages
        loop {
            if PeekMessageA(&mut msg, window.hwnd(), 0, 0, PM_REMOVE) > 0 {
                TranslateMessage(&msg);
                DispatchMessageA(&msg);

                if msg.message == WM_QUIT {
                    break;
                }
            }
        }
    }
}
