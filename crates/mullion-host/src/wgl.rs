//! WGL presentation: the core rasterizes RGBA8 into a framebuffer; this
//! module uploads it as a GL texture and draws one fullscreen quad through
//! the compatibility profile (GL 1.1 fixed-function, client-side arrays).
//! System API declarations are the documented exception to pure Rust.

use mullion::render::Framebuffer;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::OpenGL::{
    SwapBuffers, wglCreateContext, wglGetProcAddress, wglMakeCurrent,
};

#[allow(non_snake_case)]
#[link(name = "opengl32")]
unsafe extern "system" {
    fn glViewport(x: i32, y: i32, width: i32, height: i32);
    fn glMatrixMode(mode: u32);
    fn glLoadIdentity();
    fn glGenTextures(n: i32, textures: *mut u32);
    fn glBindTexture(target: u32, texture: u32);
    fn glTexParameteri(target: u32, pname: u32, param: i32);
    fn glTexImage2D(
        target: u32,
        level: i32,
        internalformat: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        typ: u32,
        pixels: *const core::ffi::c_void,
    );
    fn glTexSubImage2D(
        target: u32,
        level: i32,
        xoffset: i32,
        yoffset: i32,
        width: i32,
        height: i32,
        format: u32,
        typ: u32,
        pixels: *const core::ffi::c_void,
    );
    fn glEnable(cap: u32);
    fn glEnableClientState(array: u32);
    fn glVertexPointer(size: i32, typ: u32, stride: i32, pointer: *const core::ffi::c_void);
    fn glTexCoordPointer(size: i32, typ: u32, stride: i32, pointer: *const core::ffi::c_void);
    fn glDrawArrays(mode: u32, first: i32, count: i32);
    fn glGetError() -> u32;
}

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_NEAREST: i32 = 0x2600;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_PROJECTION: u32 = 0x1701;
const GL_MODELVIEW: u32 = 0x1700;
const GL_VERTEX_ARRAY: u32 = 0x8074;
const GL_TEXTURE_COORD_ARRAY: u32 = 0x8078;
const GL_TRIANGLE_STRIP: u32 = 0x0005;

/// Owns the GL context and the upload texture. One per window.
pub struct WglPresent {
    #[allow(dead_code)]
    hdc: HDC,
    tex: u32,
    tex_w: i32,
    tex_h: i32,
}

impl WglPresent {
    /// Create a context on `hdc` (pixel format must already be set) and
    /// prepare the projection. Also requests vsync when available.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(hdc: HDC) -> Option<Self> {
        unsafe {
            let hglrc = wglCreateContext(hdc);
            if hglrc.is_null() {
                return None;
            }
            if wglMakeCurrent(hdc, hglrc) == 0 {
                return None;
            }
            // wglSwapIntervalEXT(1) if the extension exists.
            let name = b"wglSwapIntervalEXT\0";
            let proc = wglGetProcAddress(name.as_ptr());
            if let Some(f) = proc {
                let swap_interval: unsafe extern "system" fn(i32) = std::mem::transmute(f);
                swap_interval(1);
            }
            glMatrixMode(GL_PROJECTION);
            glLoadIdentity();
            glMatrixMode(GL_MODELVIEW);
            glLoadIdentity();
            let mut tex = 0u32;
            glGenTextures(1, &mut tex);
            glBindTexture(GL_TEXTURE_2D, tex);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
            glEnable(GL_TEXTURE_2D);
            glEnableClientState(GL_VERTEX_ARRAY);
            glEnableClientState(GL_TEXTURE_COORD_ARRAY);
            glGetError();
            Some(Self {
                hdc,
                tex,
                tex_w: 0,
                tex_h: 0,
            })
        }
    }

    /// Upload `fb` and present it as a fullscreen quad.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn present(&mut self, hdc: HDC, fb: &Framebuffer) {
        unsafe {
            let w = fb.w as i32;
            let h = fb.h as i32;
            if w == 0 || h == 0 {
                return;
            }
            glViewport(0, 0, w, h);
            glBindTexture(GL_TEXTURE_2D, self.tex);
            if w != self.tex_w || h != self.tex_h {
                // Initial data; flip rows so the first framebuffer row ends
                // up at the top of the screen.
                glTexImage2D(
                    GL_TEXTURE_2D,
                    0,
                    GL_RGBA as i32,
                    w,
                    h,
                    0,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    fb.px.as_ptr() as *const core::ffi::c_void,
                );
                self.tex_w = w;
                self.tex_h = h;
            } else {
                glTexSubImage2D(
                    GL_TEXTURE_2D,
                    0,
                    0,
                    0,
                    w,
                    h,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    fb.px.as_ptr() as *const core::ffi::c_void,
                );
            }
            // GL's default 2D texture memory layout is bottom-up relative to
            // window space; rendering an upright quad with flipped V coords
            // displays the framebuffer top-down without row copies.
            #[repr(C)]
            struct Vert {
                x: f32,
                y: f32,
                u: f32,
                v: f32,
            }
            let verts = [
                Vert {
                    x: -1.0,
                    y: -1.0,
                    u: 0.0,
                    v: 1.0,
                },
                Vert {
                    x: 1.0,
                    y: -1.0,
                    u: 1.0,
                    v: 1.0,
                },
                Vert {
                    x: -1.0,
                    y: 1.0,
                    u: 0.0,
                    v: 0.0,
                },
                Vert {
                    x: 1.0,
                    y: 1.0,
                    u: 1.0,
                    v: 0.0,
                },
            ];
            glVertexPointer(
                2,
                0x1406, /* GL_FLOAT */
                16,
                verts.as_ptr() as *const core::ffi::c_void,
            );
            let uv = verts.as_ptr().cast::<u8>().add(8);
            glTexCoordPointer(2, 0x1406, 16, uv as *const core::ffi::c_void);
            glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        }
        unsafe {
            SwapBuffers(hdc);
        }
    }
}
