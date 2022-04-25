use std::{ffi::OsStr, os::windows::prelude::OsStrExt};

use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{
        BeginPaint, CreateFontW, DeleteObject, EndPaint, FillRect, SelectObject, SetBkMode,
        SetTextColor, TextOutW, BACKGROUND_MODE, FW_REGULAR, HDC, HFONT, PAINTSTRUCT,
    },
};

use super::gdi::Color;

fn prepare_string(text: &str) -> Vec<u16> {
    let mut s: Vec<u16> = OsStr::new(text).encode_wide().collect();
    s.push(0);
    s
}

pub(crate) trait GdiObject {
    /// Frees all system resources associated with this font, bitmap, brush etc.
    fn delete(self);
}

pub(crate) struct Font {
    handle: HFONT,
}

impl Font {
    pub(crate) fn create(name: &str, height: i32) -> Self {
        let font_name = prepare_string(name);
        Self {
            handle: unsafe {
                CreateFontW(
                    height,
                    0,
                    0,
                    0,
                    FW_REGULAR as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    2,
                    0,
                    font_name.as_ptr(),
                )
            },
        }
    }
}

impl GdiObject for Font {
    fn delete(self) {
        unsafe { DeleteObject(self.handle) };
    }
}

pub(crate) struct DeviceContext {
    hwnd: HWND,
    ps: PAINTSTRUCT,
    hdc: HDC,
}

impl DeviceContext {
    /// Prepares the specified window for painting.
    /// The returned context object implicitly calls EndPaint when dropped.
    pub(crate) fn paint(hwnd: HWND) -> Self {
        let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
        let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
        Self { hwnd, ps, hdc }
    }

    pub(crate) fn set_background_mix_mode(&self, mode: BACKGROUND_MODE) {
        unsafe { SetBkMode(self.hdc, mode) };
    }

    pub(crate) fn set_text_color(&self, color: Color) {
        unsafe { SetTextColor(self.hdc, color.into()) };
    }

    pub(crate) fn text_out(&self, x: i32, y: i32, raw_text: &str) {
        let text = prepare_string(raw_text);
        unsafe { TextOutW(self.hdc, x, y, text.as_ptr(), text.len() as i32 - 1) };
    }

    pub(crate) fn select_font(&self, font: &Font) -> Font {
        Font {
            handle: unsafe { SelectObject(self.hdc, font.handle) } as HFONT,
        }
    }

    pub(crate) fn fill_rect(&self, brush: isize) {
        unsafe { FillRect(self.hdc, &self.ps.rcPaint, brush) };
    }
}

impl Drop for DeviceContext {
    fn drop(&mut self) {
        unsafe { EndPaint(self.hwnd, &mut self.ps) };
    }
}
