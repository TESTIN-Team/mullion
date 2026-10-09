//! Straight-alpha sRGB color with deterministic software blending.

/// 8-bit-per-channel straight alpha color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    pub const BLACK: Color = Color::rgba(0, 0, 0, 255);
    pub const WHITE: Color = Color::rgba(255, 255, 255, 255);

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    /// From `0xRRGGBB` (opaque).
    pub const fn from_hex(hex: u32) -> Self {
        Color::rgba(
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
            255,
        )
    }

    /// From `0xRRGGBBAA`.
    pub const fn from_hex8(hex: u32) -> Self {
        Color::rgba(
            (hex >> 24) as u8,
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        )
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub fn is_opaque(self) -> bool {
        self.a == 255
    }

    /// Linear interpolation in sRGB space (good enough for UI chrome).
    pub fn lerp(self, o: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let f = |a: u8, b: u8| {
            (a as f32 + (b as f32 - a as f32) * t)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Color::rgba(
            f(self.r, o.r),
            f(self.g, o.g),
            f(self.b, o.b),
            f(self.a, o.a),
        )
    }
}

/// Source-over composite of `src` onto `dst` (straight alpha).
pub fn over(dst: Color, src: Color) -> Color {
    if src.a == 0 {
        return dst;
    }
    if src.a == 255 {
        return src;
    }
    let sa = src.a as f32 / 255.0;
    let da = dst.a as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return Color::TRANSPARENT;
    }
    let f = |s: u8, d: u8| {
        ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(
        f(src.r, dst.r),
        f(src.g, dst.g),
        f(src.b, dst.b),
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// Blend `src` onto `dst` scaled by an extra coverage factor in `[0, 1]` (anti-aliasing).
pub fn blend_cov(dst: Color, src: Color, cov: f32) -> Color {
    if cov <= 0.0 {
        return dst;
    }
    if cov >= 1.0 {
        return over(dst, src);
    }
    over(
        dst,
        src.with_alpha(((src.a as f32) * cov).round().clamp(0.0, 255.0) as u8),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing() {
        assert_eq!(Color::from_hex(0xff0000), Color::rgb(255, 0, 0));
        assert_eq!(Color::from_hex8(0x00ff0080), Color::rgba(0, 255, 0, 128));
    }

    #[test]
    fn opaque_src_replaces() {
        let dst = Color::rgb(10, 20, 30);
        let src = Color::rgb(200, 100, 50);
        assert_eq!(over(dst, src), src);
    }

    #[test]
    fn transparent_src_keeps_dst() {
        let dst = Color::rgb(10, 20, 30);
        let src = Color::rgb(255, 0, 0).with_alpha(0);
        assert_eq!(over(dst, src), dst);
    }

    #[test]
    fn half_blend_is_midpoint() {
        let dst = Color::rgb(0, 0, 0);
        let src = Color::rgb(100, 200, 255).with_alpha(128);
        let out = over(dst, src);
        // straight-alpha over opaque black: c = s * (128/255)
        assert!((out.r as i32 - 50).abs() <= 1, "r={}", out.r);
        assert!((out.g as i32 - 100).abs() <= 1, "g={}", out.g);
        assert_eq!(out.a, 255);
    }

    #[test]
    fn coverage_scales_alpha() {
        let src = Color::WHITE;
        let half = blend_cov(Color::TRANSPARENT, src, 0.5);
        assert_eq!(half.a, 128);
        // Over an opaque destination the result stays opaque (gray mix).
        let mix = blend_cov(Color::BLACK, src, 0.5);
        assert_eq!(mix.a, 255);
        assert!((mix.r as i32 - 128).abs() <= 1);
    }
}
