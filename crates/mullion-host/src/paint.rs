//! egui tessellation -> GL 1.1 painter. Applies egui's [`TexturesDelta`]
//! (the font atlas arrives there as `Managed(0)`), then draws each
//! [`egui::ClippedPrimitive`] with fixed-function client arrays
//! (position / texcoord / color), one scissored draw per clip rect, and
//! premultiplied blending. System API declarations are the documented
//! exception to pure Rust.

use std::collections::HashMap;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::OpenGL::SwapBuffers;

#[allow(non_snake_case)]
#[link(name = "opengl32")]
unsafe extern "system" {
    fn glViewport(x: i32, y: i32, width: i32, height: i32);
    fn glMatrixMode(mode: u32);
    fn glLoadIdentity();
    fn glOrtho(l: f64, r: f64, b: f64, t: f64, n: f64, f: f64);
    fn glGenTextures(n: i32, textures: *mut u32);
    fn glDeleteTextures(n: i32, textures: *const u32);
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
    fn glDisable(cap: u32);
    fn glBlendFunc(sfactor: u32, dfactor: u32);
    fn glScissor(x: i32, y: i32, width: i32, height: i32);
    fn glEnableClientState(array: u32);
    fn glVertexPointer(size: i32, typ: u32, stride: i32, pointer: *const core::ffi::c_void);
    fn glTexCoordPointer(size: i32, typ: u32, stride: i32, pointer: *const core::ffi::c_void);
    fn glColorPointer(size: i32, typ: u32, stride: i32, pointer: *const core::ffi::c_void);
    fn glDrawElements(mode: u32, count: i32, typ: u32, indices: *const core::ffi::c_void);
    fn glReadPixels(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: u32,
        typ: u32,
        data: *mut core::ffi::c_void,
    );
    fn glGetError() -> u32;
}

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_NEAREST: i32 = 0x2600;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_FLOAT: u32 = 0x1406;
const GL_UNSIGNED_INT: u32 = 0x1405;
const GL_PROJECTION: u32 = 0x1701;
const GL_MODELVIEW: u32 = 0x1700;
const GL_BLEND: u32 = 0x0BE2;
const GL_SCISSOR_TEST: u32 = 0x0C11;
const GL_ONE: u32 = 1;
const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
const GL_VERTEX_ARRAY: u32 = 0x8074;
const GL_TEXTURE_COORD_ARRAY: u32 = 0x8078;
const GL_COLOR_ARRAY: u32 = 0x8076;
const GL_TRIANGLES: u32 = 0x0004;

fn tex_key(id: egui::TextureId) -> u64 {
    match id {
        egui::TextureId::Managed(t) => t,
        egui::TextureId::User(t) => t,
    }
}

/// Owns the GL textures for egui's texture manager. One per window.
pub struct EguiPainter {
    textures: HashMap<u64, u32>,
}

impl EguiPainter {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    /// Apply an egui [`egui::TexturesDelta`] before painting.
    pub fn set_textures(&mut self, delta: &egui::TexturesDelta) {
        for (id, image_delta) in &delta.set {
            let key = tex_key(*id);
            let egui::ImageData::Color(image) = &image_delta.image;
            let [w, h] = image.size;
            let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
            for c in &image.pixels {
                rgba.extend_from_slice(&c.to_array());
            }
            unsafe {
                let tex = *self.textures.entry(key).or_insert_with(|| {
                    let mut t = 0u32;
                    glGenTextures(1, &mut t);
                    t
                });
                glBindTexture(GL_TEXTURE_2D, tex);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
                match image_delta.pos {
                    None => glTexImage2D(
                        GL_TEXTURE_2D,
                        0,
                        GL_RGBA as i32,
                        w as i32,
                        h as i32,
                        0,
                        GL_RGBA,
                        GL_UNSIGNED_BYTE,
                        rgba.as_ptr() as *const core::ffi::c_void,
                    ),
                    Some([x, y]) => glTexSubImage2D(
                        GL_TEXTURE_2D,
                        0,
                        x as i32,
                        y as i32,
                        w as i32,
                        h as i32,
                        GL_RGBA,
                        GL_UNSIGNED_BYTE,
                        rgba.as_ptr() as *const core::ffi::c_void,
                    ),
                }
            }
        }
        for id in &delta.free {
            if let Some(tex) = self.textures.remove(&tex_key(*id)) {
                unsafe { glDeleteTextures(1, &tex) };
            }
        }
    }

    /// Paint one frame of tessellated egui output, optionally capture the
    /// backbuffer (top-down RGBA, for smoke receipts), then swap.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn paint_and_swap(
        &mut self,
        hdc: HDC,
        primitives: &[egui::ClippedPrimitive],
        fb: (i32, i32),
        capture: Option<&mut Vec<u8>>,
    ) {
        let (w, h) = fb;
        if w <= 0 || h <= 0 {
            return;
        }
        unsafe {
            glViewport(0, 0, w, h);
            glMatrixMode(GL_PROJECTION);
            glLoadIdentity();
            // egui screen coordinates: origin top-left, y down, physical px.
            glOrtho(0.0, w as f64, h as f64, 0.0, -1.0, 1.0);
            glMatrixMode(GL_MODELVIEW);
            glLoadIdentity();
            glEnable(GL_TEXTURE_2D);
            glEnable(GL_BLEND);
            glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
            glEnableClientState(GL_VERTEX_ARRAY);
            glEnableClientState(GL_TEXTURE_COORD_ARRAY);
            glEnableClientState(GL_COLOR_ARRAY);
        }

        for prim in primitives {
            let clip = prim.clip_rect;
            let cx = clip.left().floor().max(0.0) as i32;
            let cy = clip.top().floor().max(0.0) as i32;
            let cw = (clip.right().ceil() - clip.left().floor()).max(0.0) as i32;
            let ch = (clip.bottom().ceil() - clip.top().floor()).max(0.0) as i32;
            let cw = cw.min(w - cx.min(w));
            let ch = ch.min(h - cy.min(h));
            if cw <= 0 || ch <= 0 {
                continue;
            }
            unsafe {
                // GL scissor origin is bottom-left; egui's is top-left.
                glScissor(cx, h - (cy + ch), cw, ch);
                glEnable(GL_SCISSOR_TEST);
            }
            // Only meshes occur for regular UI (callbacks are unused here).
            let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive else {
                unsafe { glDisable(GL_SCISSOR_TEST) };
                continue;
            };
            let Some(&tex) = self.textures.get(&tex_key(mesh.texture_id)) else {
                unsafe { glDisable(GL_SCISSOR_TEST) };
                continue;
            };
            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                unsafe { glDisable(GL_SCISSOR_TEST) };
                continue;
            }
            unsafe {
                glBindTexture(GL_TEXTURE_2D, tex);
                // epaint::Vertex is #[repr(C)] { pos: [f32;2], uv: [f32;2],
                // color: Color32 }.
                let stride = std::mem::size_of::<egui::epaint::Vertex>() as i32;
                let base = mesh.vertices.as_ptr() as *const core::ffi::c_void;
                glVertexPointer(2, GL_FLOAT, stride, base);
                glTexCoordPointer(
                    2,
                    GL_FLOAT,
                    stride,
                    (base as *const u8).add(8) as *const core::ffi::c_void,
                );
                glColorPointer(
                    4,
                    GL_UNSIGNED_BYTE,
                    stride,
                    (base as *const u8).add(16) as *const core::ffi::c_void,
                );
                glDrawElements(
                    GL_TRIANGLES,
                    mesh.indices.len() as i32,
                    GL_UNSIGNED_INT,
                    mesh.indices.as_ptr() as *const core::ffi::c_void,
                );
                glDisable(GL_SCISSOR_TEST);
            }
        }

        if let Some(out) = capture {
            out.resize((w * h * 4) as usize, 0);
            unsafe {
                glReadPixels(
                    0,
                    0,
                    w,
                    h,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    out.as_mut_ptr() as *mut core::ffi::c_void,
                );
            }
            flip_rows_vertically(out, w as usize, h as usize);
        }
        unsafe {
            glGetError();
            SwapBuffers(hdc);
        }
    }
}

impl Default for EguiPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EguiPainter {
    fn drop(&mut self) {
        for (_, tex) in self.textures.drain() {
            unsafe { glDeleteTextures(1, &tex) };
        }
    }
}

fn flip_rows_vertically(px: &mut [u8], w: usize, h: usize) {
    let stride = w * 4;
    let mut top = 0usize;
    let mut bottom = (h - 1) * stride;
    while top < bottom {
        for i in 0..stride {
            px.swap(top + i, bottom + i);
        }
        top += stride;
        bottom = bottom.wrapping_sub(stride);
    }
}
