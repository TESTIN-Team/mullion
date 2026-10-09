//! CPU rasterizer: resolves a [`Display`] into an RGBA8 [`Framebuffer`].
//! Axis-aligned fills use analytic per-pixel coverage (exact, fast);
//! rounded rects, strokes and lines use a signed-distance field with 2x2
//! supersampling; glyphs are alpha-blitted from the shaper cache.
//!
//! The rasterizer is pure: identical inputs produce identical bytes, which
//! the smoke harness and tests rely on.

use crate::color::{Color, blend_cov};
use crate::draw::{Cmd, Display, Stroke};
use crate::geometry::Rect;
use crate::text::Shaper;

/// Straight-alpha RGBA8 image, row-major, 4 bytes per pixel.
pub struct Framebuffer {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Framebuffer {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            px: vec![0; (w as usize) * (h as usize) * 4],
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if self.w == w && self.h == h {
            return;
        }
        *self = Self::new(w, h);
    }

    pub fn clear(&mut self, c: Color) {
        for px in self.px.chunks_exact_mut(4) {
            px[0] = c.r;
            px[1] = c.g;
            px[2] = c.b;
            px[3] = c.a;
        }
    }

    #[inline]
    fn blend(&mut self, x: i32, y: i32, c: Color, cov: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 || cov <= 0.0 {
            return;
        }
        let i = (y as usize * self.w as usize + x as usize) * 4;
        let dst = Color::rgba(self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]);
        let out = blend_cov(dst, c, cov);
        self.px[i] = out.r;
        self.px[i + 1] = out.g;
        self.px[i + 2] = out.b;
        self.px[i + 3] = out.a;
    }

    pub fn pixel(&self, x: u32, y: u32) -> Color {
        if x >= self.w || y >= self.h {
            return Color::TRANSPARENT;
        }
        let i = (y as usize * self.w as usize + x as usize) * 4;
        Color::rgba(self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3])
    }

    /// Number of pixels whose alpha channel is non-zero.
    pub fn opaque_pixels(&self) -> usize {
        self.px.chunks_exact(4).filter(|c| c[3] != 0).count()
    }

    /// 64-bit FNV-1a over the raw bytes, for determinism checks.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in &self.px {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// Render one frame. Clears with `display.clear` (opaque black if absent).
pub fn render(display: &Display, shaper: &Shaper, fb: &mut Framebuffer) {
    fb.clear(display.clear.unwrap_or(Color::BLACK));
    // Degenerate rect used when a clip intersection is empty: every raster
    // path no-ops on it.
    const EMPTY: Rect = Rect {
        min: crate::geometry::Vec2::new(1.0, 1.0),
        max: crate::geometry::Vec2::new(0.0, 0.0),
    };
    for layer in &display.layers {
        // Clip stack per layer: PopClip restores the enclosing clip, so
        // nested clips (scroll area inside a window, edit inside a scroll)
        // stay correctly nested.
        let mut clip_stack: Vec<Rect> = Vec::new();
        let mut clip = Rect::EVERYTHING;
        for cmd in &layer.cmds {
            match cmd {
                Cmd::Rect {
                    rect,
                    fill,
                    stroke,
                    rounding,
                } => {
                    if fill.is_some() || stroke.is_some() {
                        rounded_rect(
                            fb,
                            clip,
                            *rect,
                            *rounding,
                            *fill,
                            stroke.map(|s| (s.color, s.width)),
                        );
                    }
                }
                Cmd::Line { from, to, stroke } => {
                    draw_line(fb, clip, *from, *to, stroke);
                }
                Cmd::Text { layout, pos, color } => {
                    draw_text(fb, clip, shaper, layout, *pos, *color);
                }
                Cmd::PushClip(r) => {
                    clip_stack.push(clip);
                    clip = clip.intersect(*r).unwrap_or(EMPTY);
                }
                Cmd::PopClip => clip = clip_stack.pop().unwrap_or(Rect::EVERYTHING),
            }
        }
    }
}

/// Overlap length of `[a0, a1)` with `[b0, b1)`.
#[inline]
fn overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}

/// Exact-coverage axis-aligned fill.
pub fn fill_rect(fb: &mut Framebuffer, clip: Rect, rect: Rect, c: Color) {
    let x0 = rect.min.x.max(clip.min.x);
    let x1 = rect.max.x.min(clip.max.x);
    let y0 = rect.min.y.max(clip.min.y);
    let y1 = rect.max.y.min(clip.max.y);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let x_start = x0.floor() as i32;
    let x_end = x1.ceil() as i32;
    let y_start = y0.floor() as i32;
    let y_end = y1.ceil() as i32;
    for y in y_start..y_end {
        let cov_y = overlap(y as f32, y as f32 + 1.0, y0, y1);
        if cov_y <= 0.0 {
            continue;
        }
        for x in x_start..x_end {
            let cov_x = overlap(x as f32, x as f32 + 1.0, x0, x1);
            let cov = cov_x * cov_y;
            if cov > 0.0 {
                fb.blend(x, y, c, cov);
            }
        }
    }
}

/// Signed distance to a rounded-rectangle border (negative inside).
fn sd_rounded(p: (f32, f32), rect: Rect, r: f32) -> f32 {
    let cx = (rect.min.x + rect.max.x) * 0.5;
    let cy = (rect.min.y + rect.max.y) * 0.5;
    let hx = (rect.width() * 0.5 - r).max(0.0);
    let hy = (rect.height() * 0.5 - r).max(0.0);
    let qx = (p.0 - cx).abs() - hx;
    let qy = (p.1 - cy).abs() - hy;
    let dx = qx.max(0.0);
    let dy = qy.max(0.0);
    (dx * dx + dy * dy).sqrt() + qx.max(qy).min(0.0) - r
}

/// Fill and/or stroke a (possibly rounded) rect via SDF, 2x2 supersampled.
pub fn rounded_rect(
    fb: &mut Framebuffer,
    clip: Rect,
    rect: Rect,
    rounding: f32,
    fill: Option<Color>,
    stroke: Option<(Color, f32)>,
) {
    let r = rounding
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5)
        .max(0.0);
    let sw = stroke.map(|(_, w)| w).unwrap_or(0.0);

    // Fast path: no rounding, fill only, no stroke.
    if r < 0.5 && stroke.is_none() {
        if let Some(c) = fill {
            fill_rect(fb, clip, rect, c);
        }
        return;
    }
    // No rounding but a stroke: fill + four edge rects.
    if r < 0.5 {
        if let Some(c) = fill {
            fill_rect(fb, clip, rect, c);
        }
        if let Some((c, w)) = stroke {
            let t = (w / 2.0).ceil().max(0.5);
            fill_rect(
                fb,
                clip,
                Rect::from_xywh(rect.min.x, rect.min.y, rect.width(), w),
                c,
            );
            fill_rect(
                fb,
                clip,
                Rect::from_xywh(rect.min.x, rect.max.y - w, rect.width(), w),
                c,
            );
            fill_rect(
                fb,
                clip,
                Rect::from_xywh(rect.min.x, rect.min.y + t, w, rect.height() - 2.0 * t),
                c,
            );
            fill_rect(
                fb,
                clip,
                Rect::from_xywh(rect.max.x - w, rect.min.y + t, w, rect.height() - 2.0 * t),
                c,
            );
        }
        return;
    }

    let grow = sw + 1.0;
    let bbox = Rect {
        min: rect.min,
        max: rect.max,
    }
    .inflate(grow);
    let x0 = bbox.min.x.max(clip.min.x).floor() as i32;
    let x1 = bbox.max.x.min(clip.max.x).ceil() as i32;
    let y0 = bbox.min.y.max(clip.min.y).floor() as i32;
    let y1 = bbox.max.y.min(clip.max.y).ceil() as i32;

    for y in y0..y1 {
        for x in x0..x1 {
            let mut fill_cov = 0.0f32;
            let mut stroke_cov = 0.0f32;
            for sy in [0.25f32, 0.75f32] {
                for sx in [0.25f32, 0.75f32] {
                    let p = (x as f32 + sx, y as f32 + sy);
                    let d = sd_rounded(p, rect, r);
                    fill_cov += (0.5 - d).clamp(0.0, 1.0);
                    if stroke.is_some() {
                        stroke_cov += (0.5 - (d.abs() - sw * 0.5)).clamp(0.0, 1.0);
                    }
                }
            }
            fill_cov *= 0.25;
            stroke_cov *= 0.25;
            if let Some(c) = fill {
                fb.blend(x, y, c, fill_cov);
            }
            if let Some((c, _)) = stroke {
                fb.blend(x, y, c, stroke_cov);
            }
        }
    }
}

/// Line segment with round caps, 2x2 supersampled.
pub fn draw_line(
    fb: &mut Framebuffer,
    clip: Rect,
    from: crate::geometry::Vec2,
    to: crate::geometry::Vec2,
    s: &Stroke,
) {
    let (x0, y0) = (from.x, from.y);
    let (x1, y1) = (to.x, to.y);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = dx * dx + dy * dy;
    let half = s.width * 0.5;
    let min_x = x0.min(x1) - half - 1.0;
    let max_x = x0.max(x1) + half + 1.0;
    let min_y = y0.min(y1) - half - 1.0;
    let max_y = y0.max(y1) + half + 1.0;
    let bx0 = min_x.max(clip.min.x).floor() as i32;
    let bx1 = max_x.min(clip.max.x).ceil() as i32;
    let by0 = min_y.max(clip.min.y).floor() as i32;
    let by1 = max_y.min(clip.max.y).ceil() as i32;

    for y in by0..by1 {
        for x in bx0..bx1 {
            let mut cov = 0.0f32;
            for sy in [0.25f32, 0.75f32] {
                for sx in [0.25f32, 0.75f32] {
                    let px = x as f32 + sx;
                    let py = y as f32 + sy;
                    let t = if len2 > 0.0 {
                        ((px - x0) * dx + (py - y0) * dy) / len2
                    } else {
                        0.0
                    };
                    let t = t.clamp(0.0, 1.0);
                    let cx = x0 + t * dx;
                    let cy = y0 + t * dy;
                    let d = ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt();
                    cov += (0.5 - (d - half)).clamp(0.0, 1.0);
                }
            }
            cov *= 0.25;
            fb.blend(x, y, s.color, cov);
        }
    }
}

fn draw_text(
    fb: &mut Framebuffer,
    clip: Rect,
    shaper: &Shaper,
    layout: &crate::text::TextLayout,
    pos: crate::geometry::Vec2,
    color: Color,
) {
    for g in &layout.glyphs {
        let bm = shaper.cached_glyph(g.ch, g.px);
        let gx = (pos.x + g.x + bm.left as f32).round() as i32;
        let gy = (pos.y + bm.top as f32).round() as i32;
        for row in 0..bm.height as i32 {
            let y = gy + row;
            if y < clip.min.y as i32 || y >= clip.max.y.ceil() as i32 {
                continue;
            }
            for col in 0..bm.width as i32 {
                let x = gx + col;
                if x < clip.min.x as i32 || x >= clip.max.x.ceil() as i32 {
                    continue;
                }
                let a = bm.alpha[(row * bm.width as i32 + col) as usize] as f32 / 255.0;
                fb.blend(x, y, color, a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Color = Color::rgb(255, 0, 0);
    const GREEN: Color = Color::rgb(0, 255, 0);

    #[test]
    fn fill_rect_full_and_partial_coverage() {
        let mut fb = Framebuffer::new(4, 4);
        fb.clear(Color::rgb(0, 0, 255));
        // Rect covers half of pixel 0, all of pixel 1, half of pixel 2.
        fill_rect(
            &mut fb,
            Rect::EVERYTHING,
            Rect::from_xywh(0.5, 0.0, 2.0, 1.0),
            RED,
        );
        let p0 = fb.pixel(0, 0);
        let p1 = fb.pixel(1, 0);
        let p2 = fb.pixel(2, 0);
        let p3 = fb.pixel(3, 0);
        // Analytic coverage on blue: r = 255*cov.
        assert_eq!(p0.r, 128, "half coverage");
        assert_eq!(p1.r, 255, "full coverage");
        assert_eq!(p2.r, 128, "half coverage");
        assert_eq!(p3.r, 0, "outside");
        assert_eq!(p3.b, 255, "untouched background");
        // Vertical AA too: rect covers [0.5, 2.5). Over a transparent
        // framebuffer the color channels stay pure and alpha carries the
        // coverage.
        let mut fb2 = Framebuffer::new(4, 4);
        fill_rect(
            &mut fb2,
            Rect::EVERYTHING,
            Rect::from_xywh(0.0, 0.5, 4.0, 2.0),
            GREEN,
        );
        assert_eq!(fb2.pixel(0, 0), Color::rgba(0, 255, 0, 128));
        assert_eq!(fb2.pixel(0, 1), Color::rgba(0, 255, 0, 255));
        assert_eq!(fb2.pixel(0, 2), Color::rgba(0, 255, 0, 128));
        assert_eq!(fb2.pixel(0, 3), Color::rgba(0, 0, 0, 0));
    }

    #[test]
    fn fill_rect_respects_clip() {
        let mut fb = Framebuffer::new(8, 8);
        let clip = Rect::from_xywh(2.0, 2.0, 4.0, 4.0);
        fill_rect(&mut fb, clip, Rect::from_xywh(0.0, 0.0, 8.0, 8.0), RED);
        assert_eq!(fb.pixel(1, 1).r, 0);
        assert_eq!(fb.pixel(2, 2).r, 255);
        assert_eq!(fb.pixel(5, 5).r, 255);
        assert_eq!(fb.pixel(6, 6).r, 0);
    }

    #[test]
    fn rounded_rect_interior_filled_corner_not() {
        let mut fb = Framebuffer::new(20, 20);
        fb.clear(Color::rgb(0, 0, 0));
        let rect = Rect::from_xywh(2.0, 2.0, 16.0, 16.0);
        rounded_rect(&mut fb, Rect::EVERYTHING, rect, 6.0, Some(RED), None);
        // Center fully covered.
        assert_eq!(fb.pixel(10, 10).r, 255);
        // Sharp corner (outside the rounding) not covered.
        assert_eq!(
            fb.pixel(2, 2).r,
            0,
            "rounded corner must exclude the corner pixel"
        );
        // Mid-edge covered.
        assert!(
            fb.pixel(10, 2).r > 200,
            "top edge covered, got {}",
            fb.pixel(10, 2).r
        );
    }

    #[test]
    fn stroke_only_draws_border() {
        let mut fb = Framebuffer::new(20, 20);
        fb.clear(Color::rgb(0, 0, 0));
        let rect = Rect::from_xywh(4.0, 4.0, 12.0, 12.0);
        rounded_rect(
            &mut fb,
            Rect::EVERYTHING,
            rect,
            0.0,
            None,
            Some((GREEN, 2.0)),
        );
        assert!(fb.pixel(10, 10).g == 0, "interior stays empty");
        assert!(fb.pixel(10, 4).g > 100, "border drawn");
        assert!(fb.pixel(4, 10).g > 100, "border drawn");
    }

    #[test]
    fn line_draws_horizontal() {
        let mut fb = Framebuffer::new(16, 8);
        draw_line(
            &mut fb,
            Rect::EVERYTHING,
            crate::geometry::Vec2::new(2.0, 4.0),
            crate::geometry::Vec2::new(14.0, 4.0),
            &Stroke::new(GREEN, 2.0),
        );
        assert!(fb.pixel(8, 4).g > 100);
        assert!(fb.pixel(8, 1).g == 0);
    }

    #[test]
    fn text_blits_alpha() {
        use crate::text::{FontBackend, Glyph};
        struct F;
        impl FontBackend for F {
            fn metrics(&self, _px: f32) -> crate::text::FontMetrics {
                crate::text::FontMetrics {
                    ascent: 10.0,
                    descent: 3.0,
                    line_gap: 0.0,
                }
            }
            fn glyph(&self, ch: char, px: f32) -> Option<Glyph> {
                // Solid 4x8 block, advance 6.
                Some(Glyph {
                    ch,
                    px,
                    width: 4,
                    height: 8,
                    left: 0,
                    top: -8,
                    advance: 6.0,
                    alpha: vec![255; 32],
                })
            }
        }
        let mut shaper = Shaper::new(std::sync::Arc::new(F));
        let layout = shaper.layout("ab", 15.0);
        let mut fb = Framebuffer::new(16, 16);
        fb.clear(Color::rgb(0, 0, 0));
        draw_text(
            &mut fb,
            Rect::EVERYTHING,
            &shaper,
            &layout,
            crate::geometry::Vec2::new(1.0, 12.0),
            RED,
        );
        // First glyph block at x 1..5, y 4..12.
        assert_eq!(fb.pixel(2, 8).r, 255);
        // Second glyph at x 7..11.
        assert_eq!(fb.pixel(8, 8).r, 255);
        // Gap between glyphs.
        assert_eq!(fb.pixel(6, 8).r, 0);
        // Below baseline descent area.
        assert_eq!(fb.pixel(2, 14).r, 0);
    }

    #[test]
    fn nested_clip_restores_previous_clip() {
        let mut fb = Framebuffer::new(16, 16);
        fb.clear(Color::rgb(0, 0, 0));
        let mut dl = crate::draw::DrawList::default();
        // Outer clip covers the left half.
        let outer = Rect::from_xywh(0.0, 0.0, 8.0, 16.0);
        // Inner clip (nested) covers the top-left quarter only.
        let inner = Rect::from_xywh(0.0, 0.0, 4.0, 4.0);
        dl.push_clip(outer);
        dl.push_clip(inner);
        dl.rect(
            Rect::from_xywh(0.0, 0.0, 16.0, 16.0),
            Some(GREEN),
            None,
            0.0,
        ); // only 4x4 lands
        dl.pop_clip();
        // After popping the inner clip, drawing must clip to the OUTER rect,
        // not to the whole framebuffer.
        dl.rect(Rect::from_xywh(0.0, 0.0, 16.0, 16.0), Some(RED), None, 0.0); // left half only
        dl.pop_clip();
        let display = Display {
            layers: vec![dl],
            clear: Some(Color::rgb(0, 0, 0)),
        };
        render(
            &display,
            &crate::text::Shaper::new(std::sync::Arc::new(NoFont)),
            &mut fb,
        );
        assert_eq!(
            fb.pixel(1, 1).r,
            255,
            "inner clip area gets the later red fill too"
        );
        assert_eq!(fb.pixel(1, 15).r, 255, "outer clip region filled after pop");
        assert_eq!(
            fb.pixel(12, 8).r,
            0,
            "outside the outer clip stays untouched"
        );
        assert_eq!(fb.pixel(12, 8).g, 0);
    }

    struct NoFont;
    impl crate::text::FontBackend for NoFont {
        fn metrics(&self, _px: f32) -> crate::text::FontMetrics {
            crate::text::FontMetrics {
                ascent: 10.0,
                descent: 3.0,
                line_gap: 0.0,
            }
        }
        fn glyph(&self, _ch: char, _px: f32) -> Option<crate::text::Glyph> {
            None
        }
    }

    #[test]
    fn render_is_deterministic() {
        use crate::text::{FontBackend, Glyph};
        struct F;
        impl FontBackend for F {
            fn metrics(&self, _px: f32) -> crate::text::FontMetrics {
                crate::text::FontMetrics {
                    ascent: 10.0,
                    descent: 3.0,
                    line_gap: 0.0,
                }
            }
            fn glyph(&self, ch: char, px: f32) -> Option<Glyph> {
                Some(Glyph {
                    ch,
                    px,
                    width: 5,
                    height: 10,
                    left: 0,
                    top: -8,
                    advance: 6.0,
                    alpha: vec![180; 50],
                })
            }
        }
        let shaper = Shaper::new(std::sync::Arc::new(F));
        let mut dl = crate::draw::DrawList::default();
        dl.rect(
            Rect::from_xywh(4.0, 4.0, 60.0, 30.0),
            Some(RED),
            Some(Stroke::new(GREEN, 1.0)),
            8.0,
        );
        dl.line(
            crate::geometry::Vec2::new(0.0, 0.0),
            crate::geometry::Vec2::new(100.0, 50.0),
            Stroke::new(GREEN, 2.0),
        );
        let display = Display {
            layers: vec![dl],
            clear: Some(Color::rgb(9, 9, 9)),
        };

        let mut a = Framebuffer::new(100, 50);
        let mut b = Framebuffer::new(100, 50);
        render(&display, &shaper, &mut a);
        render(&display, &shaper, &mut b);
        assert_eq!(a.hash(), b.hash());
        assert!(a.opaque_pixels() > 0);
    }
}
