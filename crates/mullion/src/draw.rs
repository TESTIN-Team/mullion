//! Retained display list produced by each frame. Commands carry physical
//! pixels; the rasterizer resolves them against a framebuffer of the same
//! size. Layers render back-to-front in push order.

use crate::color::Color;
use crate::geometry::{Rect, Vec2};
use crate::text::TextLayout;
use std::sync::Arc;

/// A single raster command.
#[derive(Clone, Debug)]
pub enum Cmd {
    /// Rectangle fill and/or border, optionally rounded.
    Rect {
        rect: Rect,
        fill: Option<Color>,
        stroke: Option<Stroke>,
        rounding: f32,
    },
    /// A line segment with thickness.
    Line {
        from: Vec2,
        to: Vec2,
        stroke: Stroke,
    },
    /// A shaped text run drawn with its baseline at `pos.y`.
    Text {
        layout: Arc<TextLayout>,
        pos: Vec2,
        color: Color,
    },
    PushClip(Rect),
    PopClip,
}

/// Border or line style.
#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

impl Stroke {
    pub fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}

/// An ordered command list with an interleaved clip stack.
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    pub cmds: Vec<Cmd>,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.cmds.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    pub fn rect(&mut self, rect: Rect, fill: Option<Color>, stroke: Option<Stroke>, rounding: f32) {
        if fill.is_none() && stroke.is_none() {
            return;
        }
        self.cmds.push(Cmd::Rect {
            rect,
            fill,
            stroke,
            rounding,
        });
    }

    pub fn line(&mut self, from: Vec2, to: Vec2, stroke: Stroke) {
        self.cmds.push(Cmd::Line { from, to, stroke });
    }

    pub fn text(&mut self, layout: Arc<TextLayout>, pos: Vec2, color: Color) {
        self.cmds.push(Cmd::Text { layout, pos, color });
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.cmds.push(Cmd::PushClip(rect));
    }

    pub fn pop_clip(&mut self) {
        self.cmds.push(Cmd::PopClip);
    }
}

/// A full frame of renderable output: layers in back-to-front order.
#[derive(Clone, Debug, Default)]
pub struct Display {
    pub layers: Vec<DrawList>,
    /// The framebuffer background color for this frame.
    pub clear: Option<Color>,
}

impl Display {
    pub fn clear(&mut self) {
        self.layers.clear();
        self.clear = None;
    }
}
