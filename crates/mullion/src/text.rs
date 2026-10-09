//! Text: font abstraction, glyph cache, per-character layout, caret and hit
//! testing. v0.1 lays out left-to-right with no shaping, kerning or bidi;
//! each char advances by its own glyph advance. Glyph bitmaps are 8-bit
//! alpha, top-left relative to the glyph origin.

use std::collections::HashMap;
use std::sync::Arc;

/// Vertical metrics for one rasterized size, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

impl FontMetrics {
    pub fn line_height(self) -> f32 {
        (self.ascent + self.descent + self.line_gap).ceil()
    }
}

/// A rasterized glyph: 8-bit alpha bitmap plus placement.
#[derive(Clone, Debug)]
pub struct Glyph {
    pub ch: char,
    pub px: f32,
    /// Bitmap size.
    pub width: u32,
    pub height: u32,
    /// Offset from the pen position to the bitmap's top-left.
    pub left: i32,
    pub top: i32,
    /// Horizontal advance to the next pen position.
    pub advance: f32,
    /// `width * height` alpha bytes, row-major.
    pub alpha: Vec<u8>,
}

impl Glyph {
    /// Deterministic fallback box used when the backend has no glyph.
    pub fn tofu(ch: char, px: f32) -> Self {
        let side = (px * 0.6).floor().max(3.0) as u32;
        let mut alpha = vec![0u8; (side * side) as usize];
        for y in 0..side {
            for x in 0..side {
                let edge = x == 0 || y == 0 || x + 1 == side || y + 1 == side;
                alpha[(y * side + x) as usize] = if edge { 200 } else { 0 };
            }
        }
        Self {
            ch,
            px,
            width: side,
            height: side,
            left: 0,
            top: -(side as i32) - ((px * 0.15) as i32),
            advance: side as f32 + 2.0,
            alpha,
        }
    }
}

/// Font provider implemented by the host (GDI/FreeType/DirectWrite/...).
pub trait FontBackend {
    /// Vertical metrics for a raster size in physical pixels.
    fn metrics(&self, px: f32) -> FontMetrics;
    /// Rasterize one character. Return `None` to fall back to tofu.
    fn glyph(&self, ch: char, px: f32) -> Option<Glyph>;
}

/// A font backend plus per-size caches. Cheap to share across frames.
pub struct Shaper {
    backend: Arc<dyn FontBackend + Send + Sync>,
    glyphs: HashMap<(char, u32), Glyph>,
    metrics: HashMap<u32, FontMetrics>,
    layouts: HashMap<(String, u32), Arc<TextLayout>>,
}

impl Shaper {
    pub fn new(backend: Arc<dyn FontBackend + Send + Sync>) -> Self {
        Self {
            backend,
            glyphs: HashMap::new(),
            metrics: HashMap::new(),
            layouts: HashMap::new(),
        }
    }

    fn key(px: f32) -> u32 {
        px.round().max(1.0) as u32
    }

    pub fn metrics(&mut self, px: f32) -> FontMetrics {
        let k = Self::key(px);
        *self
            .metrics
            .entry(k)
            .or_insert_with(|| self.backend.metrics(k as f32))
    }

    /// Cached glyph lookup; inserts on first request. Missing glyphs are
    /// cached as tofu so the rasterizer can always resolve a layout.
    pub fn glyph(&mut self, ch: char, px: f32) -> Glyph {
        let k = Self::key(px);
        if let Some(g) = self.glyphs.get(&(ch, k)) {
            return g.clone();
        }
        let g = self
            .backend
            .glyph(ch, k as f32)
            .unwrap_or_else(|| Glyph::tofu(ch, k as f32));
        self.glyphs.insert((ch, k), g.clone());
        g
    }

    /// Look up a glyph that is guaranteed to already be cached (used by the
    /// rasterizer on layouts this shaper produced).
    pub fn cached_glyph(&self, ch: char, px: u32) -> &Glyph {
        self.glyphs
            .get(&(ch, px))
            .expect("glyph was never rasterized through this shaper")
    }

    /// Shape `text` at `px` physical pixels, caching the result.
    pub fn layout(&mut self, text: &str, px: f32) -> Arc<TextLayout> {
        let k = Self::key(px);
        if let Some(l) = self.layouts.get(&(text.to_string(), k)) {
            return Arc::clone(l);
        }
        let metrics = self.metrics(k as f32);
        let mut glyphs = Vec::with_capacity(text.chars().count());
        let mut x = 0.0f32;
        for ch in text.chars() {
            let g = self.glyph(ch, k as f32);
            glyphs.push(PlacedGlyph { ch, px: k, x });
            x += g.advance;
        }
        let layout = Arc::new(TextLayout {
            text: text.to_string(),
            px: k,
            glyphs,
            width: x,
            ascent: metrics.ascent,
            descent: metrics.descent,
        });
        self.layouts
            .insert((text.to_string(), k), Arc::clone(&layout));
        layout
    }

    /// X position of the caret before the byte at `byte_idx`.
    pub fn caret_x(&mut self, layout: &TextLayout, byte_idx: usize) -> f32 {
        let idx = layout.char_index(byte_idx);
        match layout.glyphs.get(idx) {
            Some(g) => g.x,
            None => layout.width,
        }
    }

    /// Byte index of the caret when clicked at `x` (snap to nearest boundary).
    pub fn hit(&self, layout: &TextLayout, x: f32) -> usize {
        let mut best = 0usize;
        let mut best_d = f32::INFINITY;
        let mut prev_boundary = 0usize;
        for (i, g) in layout.glyphs.iter().enumerate() {
            let center = g.x + self.advance(layout, i) * 0.5;
            let d = (center - x).abs();
            if d < best_d {
                best_d = d;
                best = prev_boundary;
            }
            prev_boundary += g.ch.len_utf8();
        }
        if (layout.width - x).abs() < best_d {
            best = layout.text.len();
        }
        best
    }

    fn advance(&self, layout: &TextLayout, i: usize) -> f32 {
        let next = layout.glyphs.get(i + 1).map(|g| g.x);
        match next {
            Some(nx) => nx - layout.glyphs[i].x,
            None => layout.width - layout.glyphs[i].x,
        }
    }
}

/// One placed glyph: char, raster size and pen x (baseline coordinate).
#[derive(Clone, Copy, Debug)]
pub struct PlacedGlyph {
    pub ch: char,
    pub px: u32,
    pub x: f32,
}

/// An immutable shaped run. `ascent`/`descent` are physical pixels.
#[derive(Debug)]
pub struct TextLayout {
    pub text: String,
    pub px: u32,
    pub glyphs: Vec<PlacedGlyph>,
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
}

impl TextLayout {
    /// Map a byte index to the char index at or before it.
    pub fn char_index(&self, byte_idx: usize) -> usize {
        let byte_idx = byte_idx.min(self.text.len());
        let mut i = 0usize;
        let mut b = 0usize;
        for ch in self.text.chars() {
            if b >= byte_idx {
                break;
            }
            b += ch.len_utf8();
            i += 1;
        }
        i
    }

    /// Clamp a byte index to a char boundary at or before it.
    pub fn snap_byte(&self, byte_idx: usize) -> usize {
        let mut i = byte_idx.min(self.text.len());
        while i > 0 && !self.text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }
}

/// Vertical alignment helper for placing text inside a rect.
pub fn baseline_for(center_y: f32, ascent: f32, descent: f32) -> f32 {
    center_y + (ascent - descent) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    /// Deterministic fake font: every glyph is an 8x8 box, advance 6.
    struct Fake;

    impl FontBackend for Fake {
        fn metrics(&self, _px: f32) -> FontMetrics {
            FontMetrics {
                ascent: 10.0,
                descent: 3.0,
                line_gap: 0.0,
            }
        }
        fn glyph(&self, ch: char, px: f32) -> Option<Glyph> {
            if ch == '\u{0}' {
                return None;
            }
            Some(Glyph {
                ch,
                px,
                width: 8,
                height: 8,
                left: 0,
                top: -8,
                advance: 6.0,
                alpha: vec![255; 64],
            })
        }
    }

    fn shaper() -> Shaper {
        Shaper::new(Arc::new(Fake))
    }

    #[test]
    fn layout_widths_and_caching() {
        let mut s = shaper();
        let l = s.layout("abc", 15.0);
        assert_eq!(l.glyphs.len(), 3);
        assert!((l.width - 18.0).abs() < 1e-4);
        assert_eq!(l.glyphs[2].x, 12.0);
        // Same input must return the same cached Arc.
        let l2 = s.layout("abc", 15.0);
        assert!(Arc::ptr_eq(&l, &l2));
        // Different size is a different entry.
        let l3 = s.layout("abc", 16.0);
        assert!(!Arc::ptr_eq(&l, &l3));
    }

    #[test]
    fn missing_glyph_becomes_tofu_and_is_cached() {
        let mut s = shaper();
        let g1 = s.glyph('\u{0}', 15.0);
        assert!(g1.width >= 3);
        let g2 = s.glyph('\u{0}', 15.0);
        assert_eq!(g1.alpha, g2.alpha);
        assert_eq!(s.cached_glyph('\u{0}', 15).alpha, g1.alpha);
    }

    #[test]
    fn caret_and_hit_roundtrip() {
        let mut s = shaper();
        let l = s.layout("abcd", 15.0);
        // byte 0 -> x 0; byte 2 -> x 12 (advance 6).
        assert_eq!(s.caret_x(&l, 0), 0.0);
        assert_eq!(s.caret_x(&l, 2), 12.0);
        assert_eq!(s.caret_x(&l, l.text.len()), 24.0);
        // Hit testing snaps to the nearest boundary.
        assert_eq!(s.hit(&l, 0.5), 0);
        assert_eq!(s.hit(&l, 6.5), 1);
        assert_eq!(s.hit(&l, 25.0), 4);
    }

    #[test]
    fn hit_multibyte_is_char_aligned() {
        let mut s = shaper();
        let l = s.layout("a中b", 15.0);
        // Boundaries are 0,1,4,5 bytes.
        assert_eq!(s.hit(&l, 100.0), 5);
        assert!(l.snap_byte(3) == 1);
        assert_eq!(l.char_index(4), 2);
    }

    #[test]
    fn baseline_centers_glyph_run() {
        let b = baseline_for(50.0, 10.0, 4.0);
        assert!((b - 53.0).abs() < 1e-4);
        assert_eq!(
            Vec2::new(1.0, 2.0).add(Vec2::new(3.0, 4.0)),
            Vec2::new(4.0, 6.0)
        );
    }
}
