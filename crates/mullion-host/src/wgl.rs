//! WGL context: creation, vsync and teardown. Drawing lives in `paint.rs`.
//! System API declarations are the documented exception to pure Rust.

use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::OpenGL::{
    wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent,
};

/// Owns one GL compatibility context. Dropping unbinds and deletes it.
pub struct GlContext {
    #[allow(dead_code)]
    hdc: HDC,
    hglrc: windows_sys::Win32::Graphics::OpenGL::HGLRC,
}

impl GlContext {
    /// Create a context on `hdc` (pixel format must already be set) and
    /// make it current. Requests vsync when the extension is available.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(hdc: HDC) -> Option<Self> {
        unsafe {
            let hglrc = wglCreateContext(hdc);
            if hglrc.is_null() {
                return None;
            }
            if wglMakeCurrent(hdc, hglrc) == 0 {
                wglDeleteContext(hglrc);
                return None;
            }
            let name = b"wglSwapIntervalEXT\0";
            let proc = wglGetProcAddress(name.as_ptr());
            if let Some(f) = proc {
                let swap_interval: unsafe extern "system" fn(i32) = std::mem::transmute(f);
                swap_interval(1);
            }
            Some(Self { hdc, hglrc })
        }
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        unsafe {
            wglMakeCurrent(self.hdc, std::ptr::null_mut());
            wglDeleteContext(self.hglrc);
        }
    }
}
