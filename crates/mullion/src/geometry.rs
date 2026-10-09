//! Axis-aligned geometry primitives shared by layout, widgets and the rasterizer.

/// A point or a 2D vector in logical UI pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const ZERO: Self = Self::new(0.0, 0.0);

    // Inherent add/sub/mul mirror the operator impls so call sites can use
    // either spelling without importing traits.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k)
    }

    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl core::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::add(self, rhs)
    }
}

impl core::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::sub(self, rhs)
    }
}

/// An axis-aligned rectangle stored as minimum and maximum corners.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub const EVERYTHING: Self = Self {
        min: Vec2::new(f32::MIN, f32::MIN),
        max: Vec2::new(f32::MAX, f32::MAX),
    };

    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self {
            min,
            max: min.add(size),
        }
    }

    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::from_min_size(Vec2::new(x, y), Vec2::new(w, h))
    }

    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn size(self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }

    pub fn center(self) -> Vec2 {
        self.min.add(self.max).mul(0.5)
    }

    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x < self.max.x && p.y >= self.min.y && p.y < self.max.y
    }

    pub fn intersects(self, o: Rect) -> bool {
        self.min.x < o.max.x && o.min.x < self.max.x && self.min.y < o.max.y && o.min.y < self.max.y
    }

    /// Intersection of two rectangles; `None` when they do not overlap.
    pub fn intersect(self, o: Rect) -> Option<Rect> {
        if !self.intersects(o) {
            return None;
        }
        Some(Rect {
            min: Vec2::new(self.min.x.max(o.min.x), self.min.y.max(o.min.y)),
            max: Vec2::new(self.max.x.min(o.max.x), self.max.y.min(o.max.y)),
        })
    }

    pub fn translate(self, v: Vec2) -> Rect {
        Rect {
            min: self.min.add(v),
            max: self.max.add(v),
        }
    }

    /// Grow outward by `v` on every side.
    pub fn inflate(self, v: f32) -> Rect {
        Rect {
            min: Vec2::new(self.min.x - v, self.min.y - v),
            max: Vec2::new(self.max.x + v, self.max.y + v),
        }
    }

    /// Clamp this rectangle so it stays inside `outer`, shrinking if needed.
    pub fn clamp_to(self, outer: Rect) -> Rect {
        let min = Vec2::new(
            self.min.x.clamp(outer.min.x, outer.max.x),
            self.min.y.clamp(outer.min.y, outer.max.y),
        );
        let max = Vec2::new(
            self.max.x.clamp(min.x, outer.max.x),
            self.max.y.clamp(min.y, outer.max.y),
        );
        Rect { min, max }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_is_half_open() {
        let r = Rect::from_xywh(10.0, 10.0, 20.0, 20.0);
        assert!(r.contains(Vec2::new(10.0, 10.0)));
        assert!(r.contains(Vec2::new(29.9, 29.9)));
        assert!(!r.contains(Vec2::new(30.0, 10.0)));
        assert!(!r.contains(Vec2::new(10.0, 30.0)));
        assert!(!r.contains(Vec2::new(9.9, 15.0)));
    }

    #[test]
    fn intersect_basics() {
        let a = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let b = Rect::from_xywh(5.0, 5.0, 10.0, 10.0);
        assert_eq!(a.intersect(b), Some(Rect::from_xywh(5.0, 5.0, 5.0, 5.0)));
        let c = Rect::from_xywh(20.0, 20.0, 5.0, 5.0);
        assert_eq!(a.intersect(c), None);
    }

    #[test]
    fn translate_and_invert() {
        let r = Rect::from_xywh(1.0, 2.0, 3.0, 4.0);
        let t = r.translate(Vec2::new(10.0, 10.0));
        assert_eq!(t, Rect::from_xywh(11.0, 12.0, 3.0, 4.0));
        assert_eq!(t.translate(Vec2::new(-10.0, -10.0)), r);
    }

    #[test]
    fn clamp_keeps_inside() {
        let outer = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let r = Rect::from_xywh(-10.0, 50.0, 30.0, 200.0).clamp_to(outer);
        assert_eq!(r, Rect::from_xywh(0.0, 50.0, 20.0, 50.0));
    }
}
