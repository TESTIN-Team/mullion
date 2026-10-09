//! Themes and spacing. All values are logical pixels; multiply by `scale` for
//! physical pixels. The host sets `scale` from the monitor DPI.

use crate::color::Color;

/// Color set for one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub window_bg: Color,
    pub surface: Color,
    pub surface_hover: Color,
    pub surface_active: Color,
    pub border: Color,
    pub text: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_active: Color,
    pub on_accent: Color,
    pub scrollbar: Color,
    pub selection: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            window_bg: Color::from_hex(0x14161a),
            surface: Color::from_hex(0x1e2127),
            surface_hover: Color::from_hex(0x282c34),
            surface_active: Color::from_hex(0x2f3440),
            border: Color::from_hex(0x3a404c),
            text: Color::from_hex(0xe6e9ee),
            text_dim: Color::from_hex(0x9aa2ae),
            accent: Color::from_hex(0x4f8cff),
            accent_hover: Color::from_hex(0x6a9eff),
            accent_active: Color::from_hex(0x3d78e6),
            on_accent: Color::from_hex(0xffffff),
            scrollbar: Color::from_hex(0x4a5160),
            selection: Color::from_hex(0x2c539c),
        }
    }

    pub fn light() -> Self {
        Self {
            window_bg: Color::from_hex(0xf2f3f5),
            surface: Color::from_hex(0xffffff),
            surface_hover: Color::from_hex(0xe9ecf1),
            surface_active: Color::from_hex(0xdde2ea),
            border: Color::from_hex(0xc9cfd8),
            text: Color::from_hex(0x22262c),
            text_dim: Color::from_hex(0x6b7280),
            accent: Color::from_hex(0x2f6fed),
            accent_hover: Color::from_hex(0x4a83f0),
            accent_active: Color::from_hex(0x275cc4),
            on_accent: Color::from_hex(0xffffff),
            scrollbar: Color::from_hex(0xb4bcc8),
            selection: Color::from_hex(0xbcd3f8),
        }
    }
}

/// Spacing and sizing constants, logical pixels, on a 4px grid.
#[derive(Clone, Copy, Debug)]
pub struct Spacing {
    pub pad: f32,
    pub gap: f32,
    pub widget_h: f32,
    pub scrollbar_w: f32,
    pub title_h: f32,
    pub rounding: f32,
    pub border_w: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            pad: 12.0,
            gap: 8.0,
            widget_h: 28.0,
            scrollbar_w: 8.0,
            title_h: 32.0,
            rounding: 6.0,
            border_w: 1.0,
        }
    }
}

/// Resolved style for one frame.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub theme: Theme,
    pub spacing: Spacing,
    /// Base font size in logical pixels.
    pub font_px: f32,
    /// Physical pixels per logical pixel (monitor DPI / 96).
    pub scale: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            theme: Theme::dark(),
            spacing: Spacing::default(),
            font_px: 15.0,
            scale: 1.0,
        }
    }
}

impl Style {
    /// Convert a logical length to physical pixels.
    pub fn px(&self, logical: f32) -> f32 {
        logical * self.scale
    }

    /// Current font size in physical pixels (rasterized size).
    pub fn font_px_physical(&self) -> f32 {
        (self.font_px * self.scale).round().max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_multiplication() {
        let s = Style {
            scale: 1.5,
            ..Default::default()
        };
        assert_eq!(s.px(10.0), 15.0);
        assert_eq!(s.font_px_physical(), 23.0);
    }

    #[test]
    fn themes_differ() {
        assert_ne!(Theme::dark(), Theme::light());
    }
}
