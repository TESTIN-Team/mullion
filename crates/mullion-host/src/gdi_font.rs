//! GDI glyph rasterizer with per-family fallback.
//!
//! `GetGlyphOutlineW` is unusable for modern system fonts (Segoe UI is a
//! variable font on current Windows and GGO returns GDI_ERROR for it), so
//! glyphs are rasterized through the real text pipeline instead: each char
//! is drawn white-on-black with `ExtTextOutW` into a 32bpp top-down DIB
//! section and the red channel becomes the coverage bitmap. Memory DCs
//! render grayscale anti-aliasing (ClearType is off), which is exactly
//! what we need. Advances come from `GetCharWidth32W`, metrics from
//! `GetTextMetricsW`.
//!
//! System API use is the documented exception to the pure-Rust rule; there
//! is no third-party code involved.

use mullion::text::{FontBackend, FontMetrics, Glyph};
use std::collections::HashMap;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, ExtTextOutW, FF_DONTCARE, GetCharWidth32W, GetTextMetricsW, HDC, HFONT,
    OUT_DEFAULT_PRECIS, RGBQUAD, SelectObject, SetBkColor, SetTextAlign, SetTextColor, TA_BASELINE,
    TA_LEFT, TEXTMETRICW,
};

/// Default family chain: Latin first, CJK fallback, symbols last.
pub const DEFAULT_FAMILIES: &[&str] = &["Segoe UI", "Microsoft YaHei UI", "Segoe UI Symbol"];

struct Target {
    dc: HDC,
    pixels: *mut u8,
    w: i32,
    h: i32,
}

struct Inner {
    /// Memory DC carrying the currently selected font.
    dc: HDC,
    hfonts: HashMap<(usize, i32), HFONT>,
    selected: Option<(usize, i32)>,
    /// Raster target DIB, one at a time per size.
    target: Option<Target>,
    target_px: i32,
}

impl Inner {
    fn hfont(&mut self, family: usize, px: i32) -> Option<HFONT> {
        if let Some(h) = self.hfonts.get(&(family, px)) {
            return Some(*h);
        }
        let name: Vec<u16> = DEFAULT_FAMILIES[family].encode_utf16().chain([0]).collect();
        // Negative height = character height in pixels.
        let h = unsafe {
            CreateFontW(
                -px,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32,
                0, // DEFAULT_QUALITY
                (DEFAULT_PITCH | (FF_DONTCARE << 4)) as u32,
                name.as_ptr(),
            )
        };
        if h.is_null() {
            return None;
        }
        self.hfonts.insert((family, px), h);
        Some(h)
    }

    fn select(&mut self, family: usize, px: i32) -> bool {
        if self.selected == Some((family, px)) {
            return true;
        }
        let Some(h) = self.hfont(family, px) else {
            return false;
        };
        let old = unsafe { SelectObject(self.dc, h) };
        if old.is_null() {
            return false;
        }
        self.selected = Some((family, px));
        true
    }

    /// (Re)create the raster target DIB for `px`.
    fn ensure_target(&mut self, px: i32) -> Option<&mut Target> {
        if self.target.is_some() && self.target_px == px {
            return self.target.as_mut();
        }
        let side = (px * 6).clamp(96, 1024);
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: side,
                biHeight: -side, // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let dib = unsafe {
            CreateDIBSection(
                std::ptr::null_mut(),
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            )
        };
        if dib.is_null() || bits.is_null() {
            return None;
        }
        let target_dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        if target_dc.is_null() {
            unsafe { DeleteObject(dib) };
            return None;
        }
        unsafe {
            SelectObject(target_dc, dib);
            SetTextColor(target_dc, 0x00FF_FFFF); // white (COLORREF)
            SetBkColor(target_dc, 0x0000_0000); // black
            SetTextAlign(target_dc, TA_LEFT | TA_BASELINE);
        }
        if let Some(old) = self.target.take() {
            unsafe { DeleteDC(old.dc) };
        }
        self.target_px = px;
        self.target = Some(Target {
            dc: target_dc,
            pixels: bits as *mut u8,
            w: side,
            h: side,
        });
        self.target.as_mut()
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(t) = self.target.take() {
            unsafe { DeleteDC(t.dc) };
        }
        for (_, h) in self.hfonts.drain() {
            unsafe { DeleteObject(h) };
        }
        unsafe { DeleteDC(self.dc) };
    }
}

// The DC, DIB and fonts are used only from the UI thread; the unsafe impls
// just satisfy the `Arc<dyn FontBackend + Send + Sync>` bound in the core.
unsafe impl Send for GdiFontSet {}
unsafe impl Sync for GdiFontSet {}

/// GDI-backed [`FontBackend`] over a family fallback chain.
pub struct GdiFontSet {
    inner: std::sync::Mutex<Inner>,
}

impl GdiFontSet {
    /// Create a font set over [`DEFAULT_FAMILIES`].
    pub fn new() -> Option<Self> {
        let dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        if dc.is_null() {
            return None;
        }
        Some(Self {
            inner: std::sync::Mutex::new(Inner {
                dc,
                hfonts: HashMap::new(),
                selected: None,
                target: None,
                target_px: 0,
            }),
        })
    }
}

impl Default for GdiFontSet {
    fn default() -> Self {
        Self::new().expect("CreateCompatibleDC failed")
    }
}

impl FontBackend for GdiFontSet {
    fn metrics(&self, px: f32) -> FontMetrics {
        let px = px.round().max(1.0) as i32;
        let mut inner = self.inner.lock().unwrap();
        if !inner.select(0, px) {
            return FontMetrics {
                ascent: px as f32 * 0.8,
                descent: px as f32 * 0.2,
                line_gap: 0.0,
            };
        }
        let mut tm: TEXTMETRICW = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetTextMetricsW(inner.dc, &mut tm) };
        if ok == 0 {
            return FontMetrics {
                ascent: px as f32 * 0.8,
                descent: px as f32 * 0.2,
                line_gap: 0.0,
            };
        }
        FontMetrics {
            ascent: tm.tmAscent as f32,
            descent: tm.tmDescent as f32,
            line_gap: tm.tmExternalLeading as f32,
        }
    }

    fn glyph(&self, ch: char, px: f32) -> Option<Glyph> {
        if (ch as u32) > 0xFFFF {
            return None; // non-BMP unsupported in v0.1
        }
        let px = px.round().max(1.0) as i32;
        let mut inner = self.inner.lock().unwrap();
        let code = ch as u32;

        for family in 0..DEFAULT_FAMILIES.len() {
            if !inner.select(family, px) {
                continue;
            }
            let mut adv: i32 = 0;
            if unsafe { GetCharWidth32W(inner.dc, code, code, &mut adv) } == 0 {
                continue;
            }
            let Some(target) = inner.ensure_target(px) else {
                continue;
            };
            let (tw, th) = (target.w, target.h);
            let pen_x = (px as f32 * 1.5) as i32;
            let baseline = (px as f32 * 2.5) as i32;
            let unit: [u16; 2] = [ch as u16, 0];
            unsafe {
                let px_slice =
                    std::slice::from_raw_parts_mut(target.pixels, (tw * th * 4) as usize);
                px_slice.fill(0);
                let drawn = ExtTextOutW(
                    target.dc,
                    pen_x,
                    baseline,
                    0,
                    std::ptr::null(),
                    unit.as_ptr(),
                    1,
                    std::ptr::null(),
                );
                if drawn == 0 {
                    continue;
                }
            }
            // Extract the ink bbox from the red channel.
            let px_slice =
                unsafe { std::slice::from_raw_parts(target.pixels, (tw * th * 4) as usize) };
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for y in 0..th {
                for x in 0..tw {
                    if px_slice[((y * tw + x) * 4) as usize] != 0 {
                        if x < min_x {
                            min_x = x;
                        }
                        if y < min_y {
                            min_y = y;
                        }
                        if x > max_x {
                            max_x = x;
                        }
                        if y > max_y {
                            max_y = y;
                        }
                    }
                }
            }
            if min_x > max_x {
                // No ink: valid only for whitespace, whose advance we have.
                if ch.is_whitespace() {
                    return Some(Glyph {
                        ch,
                        px: px as f32,
                        width: 0,
                        height: 0,
                        left: 0,
                        top: 0,
                        advance: adv as f32,
                        alpha: Vec::new(),
                    });
                }
                continue; // this family lacks the glyph
            }
            let w = (max_x - min_x + 1) as u32;
            let h = (max_y - min_y + 1) as u32;
            let mut alpha = Vec::with_capacity((w * h) as usize);
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    alpha.push(px_slice[((y * tw + x) * 4) as usize]);
                }
            }
            return Some(Glyph {
                ch,
                px: px as f32,
                width: w,
                height: h,
                left: min_x - pen_x,
                // Down-positive offset from the baseline.
                top: min_y - baseline,
                advance: adv as f32,
                alpha,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gdi_metrics_and_glyph() {
        let Some(set) = GdiFontSet::new() else {
            return; // headless CI
        };
        let m = set.metrics(16.0);
        assert!(
            m.ascent > 8.0 && m.ascent < 32.0,
            "ascent out of range: {m:?}"
        );
        let latin = set.glyph('A', 16.0);
        assert!(latin.is_some(), "Latin glyph must resolve");
        let g = latin.unwrap();
        assert!(g.width > 0 && g.height > 0);
        assert!(g.alpha.iter().any(|&a| a > 100), "glyph must have ink");
        assert!(g.advance > 0.0);

        let cjk = set.glyph('中', 16.0);
        assert!(cjk.is_some(), "CJK glyph must resolve via fallback");
        let cjk = cjk.unwrap();
        assert!(cjk.width > 0 && cjk.height > 0);
        assert!(cjk.top <= 0, "CJK ink starts above the baseline");
        assert!(cjk.alpha.iter().any(|&a| a > 100));

        let space = set.glyph(' ', 16.0);
        assert!(space.is_some(), "space must resolve");
        assert_eq!(space.unwrap().alpha.len(), 0);
    }
}
