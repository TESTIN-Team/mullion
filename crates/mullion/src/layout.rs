//! Minimal linear layout helpers. Widgets get explicit rects; these make
//! splitting a body rect into rows or columns one-liners. All logical px.

use crate::geometry::{Rect, Vec2};

/// Splits a rect vertically into rows top-to-bottom.
#[derive(Clone, Copy, Debug)]
pub struct Column {
    pub rect: Rect,
    pub cursor_y: f32,
    pub spacing: f32,
}

impl Column {
    pub fn new(rect: Rect, spacing: f32) -> Self {
        Self {
            rect,
            cursor_y: rect.min.y,
            spacing,
        }
    }

    pub fn with_padding(rect: Rect, pad: f32, spacing: f32) -> Self {
        let inner = rect.inflate(-pad);
        Self::new(inner, spacing)
    }

    /// Reserve the next row of height `h` and advance.
    pub fn add(&mut self, h: f32) -> Rect {
        let r = Rect::from_min_size(
            Vec2::new(self.rect.min.x, self.cursor_y),
            Vec2::new(self.rect.width(), h),
        );
        self.cursor_y += h + self.spacing;
        r
    }

    /// Remaining height below the cursor.
    pub fn remaining(&self) -> f32 {
        (self.rect.max.y - self.cursor_y + self.spacing).max(0.0)
    }

    /// The rect covering everything below the cursor.
    pub fn remaining_rect(&self) -> Rect {
        Rect::from_min_size(
            Vec2::new(self.rect.min.x, self.cursor_y),
            Vec2::new(self.rect.width(), self.remaining()),
        )
    }
}

/// Splits a rect horizontally into columns left-to-right.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub rect: Rect,
    pub cursor_x: f32,
    pub spacing: f32,
}

impl Row {
    pub fn new(rect: Rect, spacing: f32) -> Self {
        Self {
            rect,
            cursor_x: rect.min.x,
            spacing,
        }
    }

    pub fn with_padding(rect: Rect, pad: f32, spacing: f32) -> Self {
        let inner = rect.inflate(-pad);
        Self::new(inner, spacing)
    }

    pub fn add(&mut self, w: f32) -> Rect {
        let r = Rect::from_min_size(
            Vec2::new(self.cursor_x, self.rect.min.y),
            Vec2::new(w, self.rect.height()),
        );
        self.cursor_x += w + self.spacing;
        r
    }

    pub fn remaining(&self) -> f32 {
        (self.rect.max.x - self.cursor_x + self.spacing).max(0.0)
    }
}

/// Two-column label/field rows used by forms.
pub fn form_row(rect: Rect, label_w: f32) -> (Rect, Rect) {
    let label = Rect::from_min_size(rect.min, Vec2::new(label_w, rect.height()));
    let field = Rect::from_min_size(
        Vec2::new(rect.min.x + label_w, rect.min.y),
        Vec2::new((rect.width() - label_w).max(0.0), rect.height()),
    );
    (label, field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_splits_with_spacing() {
        let mut c = Column::new(Rect::from_xywh(0.0, 0.0, 100.0, 60.0), 4.0);
        let a = c.add(10.0);
        let b = c.add(10.0);
        assert_eq!(a, Rect::from_xywh(0.0, 0.0, 100.0, 10.0));
        assert_eq!(b, Rect::from_xywh(0.0, 14.0, 100.0, 10.0));
        // Rows used 10 + 4 + 10 = 24 of 60.
        assert!((c.remaining() - 36.0).abs() < 1e-5);
    }

    #[test]
    fn row_and_padding() {
        let mut r = Row::with_padding(Rect::from_xywh(0.0, 0.0, 104.0, 30.0), 2.0, 0.0);
        let a = r.add(50.0);
        assert_eq!(a.min, Vec2::new(2.0, 2.0));
        assert!((a.width() - 50.0).abs() < 1e-5);
        assert!((r.remaining() - 50.0).abs() < 1e-5);
    }

    #[test]
    fn form_row_split() {
        let (label, field) = form_row(Rect::from_xywh(0.0, 0.0, 100.0, 20.0), 30.0);
        assert_eq!(label.width(), 30.0);
        assert_eq!(field.min.x, 30.0);
        assert_eq!(field.width(), 70.0);
    }
}
