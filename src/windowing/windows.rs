use flume::{Receiver, Sender};
use parking_lot::Mutex;
use std::{
    ffi::OsStr, os::windows::prelude::OsStrExt, sync::Arc, thread::JoinHandle, time::Duration,
};
use thiserror::Error;
use windows_sys::{
    core::PCWSTR,
    Win32::Foundation::{HWND, LPARAM},
    Win32::{
        Foundation::{HINSTANCE, LRESULT, WPARAM},
        Graphics::Gdi::{InvalidateRect, COLOR_BTNSHADOW, HBRUSH, TRANSPARENT},
        UI::WindowsAndMessaging::*,
    },
};

use crate::song::SongInfo;

use super::{
    gdi::Color,
    windows_gdi::{DeviceContext, Font, GdiObject},
    Window as WindowTrait, WindowEvent,
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

    fn try_create(name: &str) -> Result<WindowClass, ClassRegisterError> {
        WindowClass::try_register(
            name,
            WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                lpfnWndProc: Some(custom_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as HINSTANCE,
                hIcon: 0 as HICON,
                hCursor: unsafe { LoadCursorW(0, IDC_ARROW) },
                hbrBackground: COLOR_BTNSHADOW as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: std::ptr::null(),
            },
        )
    }

    fn name_ptr(&self) -> *const u16 {
        self.name.as_ptr()
    }
}

type Runnable = dyn FnOnce() -> () + Send + 'static;

#[derive(Error, Debug)]
enum WindowCreateError {
    #[error("CreateWindowExW returned zero")]
    WindowsError,
}

pub struct Window {
    hwnd: HWND,
    ui_thread_tasks: Mutex<Option<Vec<Box<Runnable>>>>,
    current_info: Mutex<Option<SongInfo>>,
}

fn get_primary_display_size() -> (i32, i32) {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    (width, height)
}

impl Window {
    fn try_create(class: &WindowClass, title: &str) -> Result<Arc<Window>, WindowCreateError> {
        unsafe {
            let (window_width, window_height) = (400, 300);
            let (screen_width, screen_height) = get_primary_display_size();

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
                0 => Err(WindowCreateError::WindowsError),
                _ => Ok(Arc::new(Window {
                    hwnd,
                    ui_thread_tasks: Mutex::new(Some(vec![])),
                    current_info: Mutex::new(None),
                })),
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

    /// Retrieves a posted message from the queue (if it exists).
    fn poll_message(&self, timeout: Duration) -> Result<MSG, ()> {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            let hwnd = 0 as HWND;
            let timer = SetTimer(hwnd, 0, timeout.as_millis() as u32, None);
            let result = GetMessageW(&mut msg, hwnd, 0, 0);
            KillTimer(hwnd, timer);
            return if result == -1 { Err(()) } else { Ok(msg) };
        }
    }

    // Runs all tasks that have been queued for the UI thread.
    fn run_queued_tasks(&self) {
        let mut guard = self.ui_thread_tasks.lock();
        let tasks = guard.replace(Vec::new()).unwrap();
        drop(guard);
        for task in tasks {
            task();
        }
    }

    fn repaint(&self) {
        let context = DeviceContext::paint(self.hwnd());

        context.fill_rect(COLOR_BTNSHADOW as HBRUSH);

        let font = Font::create("Segoe UI", 24);
        let prev_font = context.select_font(&font);

        context.set_background_mix_mode(TRANSPARENT);
        context.set_text_color(Color::rgb(0, 0, 0));

        let song: Option<SongInfo>;
        {
            let guard = self.current_info.lock();
            song = (&*guard).clone();
        }

        let song_as_text = match song {
            Some(info) => "{artist} - {title}"
                .replace("{artist}", &info.artist)
                .replace("{title}", &info.title),
            None => "...".to_string(),
        };

        context.text_out(10, 10, &song_as_text);
        context.select_font(&prev_font);
        font.delete();
    }
}

impl WindowTrait for Window {
    fn run_on_ui_thread(&self, runnable: impl FnOnce() -> () + 'static + Send) {
        let mut guard = self.ui_thread_tasks.lock();
        let tasks = &mut (*guard);
        tasks.as_mut().unwrap().push(Box::new(runnable));
    }
    fn post_repaint_request(&self) {
        unsafe { InvalidateRect(self.hwnd(), std::ptr::null(), 0) };
    }
}

unsafe extern "system" fn custom_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => 0,
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn prepare_string(text: &str) -> Vec<u16> {
    let mut s: Vec<u16> = OsStr::new(text).encode_wide().collect();
    s.push(0);
    s
}

/// Spawns a window on a separate thread.
pub fn create() -> (Arc<Window>, Receiver<WindowEvent>, Sender<Option<SongInfo>>) {
    let (ss, sr) = flume::unbounded::<Option<SongInfo>>();
    let (ws, wr) = flume::bounded::<Arc<Window>>(1);
    let (wnd_sender, wnd_recvr) = flume::unbounded::<WindowEvent>();

    let _window_thread: JoinHandle<anyhow::Result<()>> = std::thread::spawn(move || {
        let class = WindowClass::try_create("CurrentSongWindowClass")?;
        let window = Window::try_create(&class, "Current Song")?;
        ws.send(window.clone()).unwrap();
        window.show();

        while let Ok(msg) = window.poll_message(Duration::from_millis(750)) {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            match msg.message {
                WM_PAINT => window.repaint(),
                WM_QUIT => break,
                _ => {}
            }

            window.run_queued_tasks();
            match sr.try_recv() {
                Ok(o) => {
                    {
                        let mut guard = window.current_info.lock();
                        *guard = o;
                    }
                    window.post_repaint_request();
                }
                Err(_) => {}
            }
        }

        wnd_sender.send(WindowEvent::Closed).unwrap();
        Ok(())
    });

    let window = wr.recv().unwrap();
    (window, wnd_recvr, ss)
}
