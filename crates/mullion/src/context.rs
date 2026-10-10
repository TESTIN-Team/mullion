//! Frame context: owns input, memory, style, shaper and the display list
//! being built. Widgets draw and resolve interaction through `&mut Ctx`.
//!
//! Interaction model: hover and clicks are resolved against the rects
//! registered *last* frame (so the topmost widget drawn under the pointer
//! is authoritative, and a click always lands on what the user saw). The
//! pointer is captured by a widget on press and stays captured until
//! release, which is what makes sliders and text selection robust.

use crate::color::Color;
use crate::draw::{Display, DrawList, Stroke};
use crate::geometry::{Rect, Vec2};
use crate::input::{FrameInput, Key, KeyEvent};
use crate::memory::{ClickRecord, Id, Memory};
use crate::style::Style;
use crate::text::{Shaper, TextLayout};
use std::sync::Arc;

/// A second click is a double-click when it lands strictly inside both limits.
const DOUBLE_CLICK_MS: f64 = 500.0;
const DOUBLE_CLICK_SLOP: f32 = 6.0;

/// Result of resolving interaction for one widget this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Response {
    pub hovered: bool,
    /// Full press + release over the widget.
    pub clicked: bool,
    /// Second complete click within 500 ms and 6 logical pixels of the previous one.
    pub double_clicked: bool,
    /// The press happened on this widget this frame.
    pub drag_started: bool,
    /// Pointer is captured by this widget and still down.
    pub dragging: bool,
    /// Pointer movement since last frame while dragging.
    pub drag_delta: Vec2,
    /// Widget holds keyboard focus.
    pub focused: bool,
}

impl Response {
    /// Convenience for buttons: activated by click or Enter while focused.
    pub fn activated(&self) -> bool {
        self.clicked
    }
}

/// What the host should do after the frame.
#[derive(Clone, Debug, Default)]
pub struct FrameOutput {
    /// Logical caret position of the focused text field, for IME windows.
    pub ime_caret: Option<Vec2>,
    /// Text to place on the clipboard.
    pub copy_text: Option<String>,
    /// Id of the edit field that asked for a paste.
    pub paste_id: Option<Id>,
}

pub struct Ctx {
    pub input: FrameInput,
    pub memory: Memory,
    pub style: Style,
    pub shaper: Shaper,
    pub display: Display,
    cur: DrawList,
    rects: Vec<(Id, Rect)>,
    focusables: Vec<Id>,
    key_consumed: Vec<bool>,
    ime_caret: Option<Vec2>,
    copy_text: Option<String>,
    paste_id: Option<Id>,
}

impl Ctx {
    pub fn new(backend: Arc<dyn crate::text::FontBackend + Send + Sync>) -> Self {
        Self {
            input: FrameInput::default(),
            memory: Memory::default(),
            style: Style::default(),
            shaper: Shaper::new(backend),
            display: Display::default(),
            cur: DrawList::default(),
            rects: Vec::new(),
            focusables: Vec::new(),
            key_consumed: Vec::new(),
            ime_caret: None,
            copy_text: None,
            paste_id: None,
        }
    }

    /// Start a frame. `input` positions are logical pixels.
    pub fn begin(&mut self, input: FrameInput) {
        self.shaper.new_frame();
        self.input = input;
        self.key_consumed = vec![false; self.input.keys.len()];
        self.rects.clear();
        self.focusables.clear();
        self.display.clear();
        self.display.clear = Some(self.style.theme.window_bg);
        self.cur.clear();
        self.ime_caret = None;
        self.copy_text = None;
        self.paste_id = None;
        // Keep the capture alive on the release frame so the pressed widget
        // can observe `clicked`; drop it if the pointer is up without one.
        if !self.input.primary_down && !self.input.primary_released {
            self.memory.pointer_capture = None;
        }
    }

    /// Finish the frame: flush the last layer, promote this frame's
    /// registration into `memory`, move keyboard focus on Tab.
    pub fn end(&mut self) -> FrameOutput {
        if !self.cur.is_empty() || self.display.layers.is_empty() {
            self.display.layers.push(std::mem::take(&mut self.cur));
        } else {
            self.cur.clear();
        }
        self.memory.prev_rects = std::mem::take(&mut self.rects);
        self.memory.prev_focusables = std::mem::take(&mut self.focusables);
        // The release frame ends the capture — unless a new press already
        // happened in the same frame (press-release-press), in which case
        // the drag continues. last_mouse feeds next frame's drag deltas.
        if self.input.primary_released && !self.input.primary_down {
            self.memory.pointer_capture = None;
        }
        self.memory.last_mouse = self.input.mouse_pos;

        // Tab / Shift+Tab cycles focus over registered focusables.
        let tab = self.input.keys.iter().enumerate().find(|(i, _)| {
            self.input.keys[*i].key == Key::Tab
                && self.input.keys[*i].pressed
                && !self.key_consumed[*i]
        });
        if let Some((i, ev)) = tab {
            self.key_consumed[i] = true;
            if !self.memory.prev_focusables.is_empty() {
                let idx = self
                    .memory
                    .focus
                    .and_then(|f| self.memory.prev_focusables.iter().position(|&x| x == f));
                let n = self.memory.prev_focusables.len() as i64;
                let cur = idx.map(|x| x as i64).unwrap_or(-1);
                let step = if ev.mods.shift { -1 } else { 1 };
                let next = (cur + step + n) % n;
                self.memory.focus = Some(self.memory.prev_focusables[next as usize]);
            } else {
                self.memory.focus = None;
            }
        }

        FrameOutput {
            ime_caret: self.ime_caret,
            copy_text: self.copy_text.take(),
            paste_id: self.paste_id,
        }
    }

    // ---- interaction ----

    /// Resolve interaction for `id` at `rect` (logical pixels, this frame's
    /// geometry). Hit testing uses last frame's rects.
    pub fn interact(&mut self, id: Id, rect: Rect) -> Response {
        self.rects.push((id, rect));

        let hovered_id = self.topmost_at(self.input.mouse_pos);
        let hovered = hovered_id == Some(id);

        let capture = self.memory.pointer_capture;
        let mut drag_started = false;
        if self.input.primary_pressed && hovered && capture.is_none() {
            self.memory.pointer_capture = Some(id);
            drag_started = true;
        }
        // A press followed by a release inside the SAME frame must still
        // count as a click, so a capture taken during this call is as good
        // as one held from a previous frame.
        let owns_pointer = capture == Some(id) || drag_started;
        // `drag_started` counts as dragging so sliders track on the press
        // frame already.
        let dragging = owns_pointer && self.input.primary_down;
        let clicked = self.input.primary_released && owns_pointer && hovered;
        let double_clicked = self.note_click(clicked);

        let drag_delta = if dragging {
            self.input
                .mouse_pos
                .zip(self.prev_mouse())
                .map(|(cur, prev)| cur.sub(prev))
                .unwrap_or_default()
        } else {
            Vec2::ZERO
        };

        Response {
            hovered,
            clicked,
            double_clicked,
            drag_started,
            dragging,
            drag_delta,
            focused: self.memory.focus == Some(id),
        }
    }

    /// Record a completed click and report whether it pairs with the previous one.
    fn note_click(&mut self, clicked: bool) -> bool {
        if !clicked {
            return false;
        }
        let Some(pos) = self.input.mouse_pos else {
            return false;
        };
        let time_ms = self.input.time_ms;
        let double = self.memory.last_click.is_some_and(|prev| {
            let dt = time_ms - prev.time_ms;
            (0.0..DOUBLE_CLICK_MS).contains(&dt) && pos.sub(prev.pos).len() < DOUBLE_CLICK_SLOP
        });
        self.memory.last_click = Some(ClickRecord { time_ms, pos });
        double
    }

    /// Topmost widget under the pointer using last frame's rects.
    fn topmost_at(&self, pos: Option<Vec2>) -> Option<Id> {
        let p = pos?;
        self.memory
            .prev_rects
            .iter()
            .rev()
            .find(|(_, r)| r.contains(p))
            .map(|(id, _)| *id)
    }

    fn prev_mouse(&self) -> Option<Vec2> {
        self.memory.last_mouse
    }

    // ---- keyboard ----

    pub fn register_focusable(&mut self, id: Id) {
        self.focusables.push(id);
    }

    pub fn focused(&self, id: Id) -> bool {
        self.memory.focus == Some(id)
    }

    /// First unconsumed key event matching `pred`; marks it consumed.
    pub fn take_key(&mut self, pred: impl Fn(&KeyEvent) -> bool) -> Option<KeyEvent> {
        for (i, ev) in self.input.keys.iter().enumerate() {
            if !self.key_consumed[i] && pred(ev) {
                self.key_consumed[i] = true;
                return Some(*ev);
            }
        }
        None
    }

    /// Consume a press of `key` (no modifiers unless `ctrl` given).
    pub fn key_pressed(&mut self, key: Key, ctrl: bool) -> bool {
        self.take_key(|ev| ev.pressed && ev.key == key && ev.mods.ctrl == ctrl && !ev.mods.alt)
            .is_some()
    }

    // ---- IME / clipboard requests ----

    pub fn set_ime_caret(&mut self, logical_pos: Vec2) {
        self.ime_caret = Some(logical_pos);
    }

    pub fn request_copy(&mut self, text: String) {
        self.copy_text = Some(text);
    }

    pub fn request_paste(&mut self, id: Id) {
        self.paste_id = Some(id);
    }

    // ---- drawing (logical pixels in, physical pixels out) ----

    fn scale_rect(&self, r: Rect) -> Rect {
        let s = self.style.scale;
        Rect {
            min: r.min.mul(s),
            max: r.max.mul(s),
        }
    }

    /// Start a new layer (drawn above everything pushed so far).
    pub fn push_layer(&mut self) {
        self.display.layers.push(std::mem::take(&mut self.cur));
    }

    pub fn fill_stroke(
        &mut self,
        rect: Rect,
        fill: Option<Color>,
        stroke: Option<Stroke>,
        rounding: f32,
    ) {
        let s = self.style.scale;
        self.cur
            .rect(self.scale_rect(rect), fill, stroke, rounding * s);
    }

    pub fn fill(&mut self, rect: Rect, fill: Color) {
        self.fill_stroke(rect, Some(fill), None, 0.0);
    }

    pub fn line(&mut self, from: Vec2, to: Vec2, color: Color, width: f32) {
        let s = self.style.scale;
        self.cur
            .line(from.mul(s), to.mul(s), Stroke::new(color, width * s));
    }

    /// Draw a shaped layout with its baseline at `baseline.y`, starting at
    /// `baseline.x`. Positions are logical pixels.
    pub fn text_at(&mut self, layout: Arc<TextLayout>, baseline: Vec2, color: Color) {
        let s = self.style.scale;
        self.cur.text(layout, baseline.mul(s), color);
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.cur.push_clip(self.scale_rect(rect));
    }

    pub fn pop_clip(&mut self) {
        self.cur.pop_clip();
    }

    // ---- text helpers ----

    pub fn layout_text(&mut self, text: &str) -> Arc<TextLayout> {
        let px = self.style.font_px_physical();
        self.shaper.layout(text, px)
    }

    pub fn metrics(&mut self) -> crate::text::FontMetrics {
        let px = self.style.font_px_physical();
        self.shaper.metrics(px)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    struct StubBackend;

    impl crate::text::FontBackend for StubBackend {
        fn metrics(&self, _px: f32) -> crate::text::FontMetrics {
            crate::text::FontMetrics {
                ascent: 10.0,
                descent: 3.0,
                line_gap: 0.0,
            }
        }
        fn glyph(&self, ch: char, px: f32) -> Option<crate::text::Glyph> {
            Some(crate::text::Glyph {
                ch,
                px,
                width: 4,
                height: 8,
                left: 0,
                top: -7,
                advance: 5.0,
                alpha: vec![255; 32],
            })
        }
    }

    fn ctx() -> Ctx {
        Ctx::new(std::sync::Arc::new(StubBackend))
    }

    fn frame_input(mouse: Option<Vec2>, pressed: bool, released: bool) -> FrameInput {
        FrameInput {
            mouse_pos: mouse,
            primary_pressed: pressed,
            primary_released: released,
            primary_down: pressed || (!released && mouse.is_some()),
            ..Default::default()
        }
    }

    const BTN: Id = 42;

    fn btn_rect() -> Rect {
        Rect::from_xywh(0.0, 0.0, 100.0, 30.0)
    }

    /// Two-frame pattern: register the rect, end, then interact next frame.
    #[test]
    fn click_requires_hover_on_press_and_release() {
        let mut c = ctx();

        // Frame 1: widget appears.
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        // Frame 2: press inside.
        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), true, false));
        let r = c.interact(BTN, btn_rect());
        assert!(r.hovered);
        assert!(r.drag_started);
        assert!(!r.clicked);
        c.end();

        // Frame 3: release inside -> click.
        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), false, true));
        let r = c.interact(BTN, btn_rect());
        assert!(r.clicked);
        assert!(!r.dragging);
        c.end();

        // Frame 4: capture released; a later press elsewhere doesn't click.
        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), false, false));
        c.interact(BTN, btn_rect());
        c.end();
    }

    #[test]
    fn click_outside_on_release_is_not_clicked() {
        let mut c = ctx();
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), true, false));
        c.interact(BTN, btn_rect());
        c.end();

        // Release outside the widget: capture ends, no click.
        c.begin(frame_input(Some(Vec2::new(500.0, 500.0)), false, true));
        let r = c.interact(BTN, btn_rect());
        assert!(!r.clicked);
        c.end();
    }

    #[test]
    fn later_rect_steals_hover() {
        let mut c = ctx();
        let behind = Id::from(1u8);
        let front = Id::from(2u8);
        // Frame 1 registers both rects.
        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), false, false));
        c.interact(behind, btn_rect());
        c.interact(front, btn_rect());
        c.end();
        // Frame 2 resolves hover: the later (topmost) rect wins.
        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), false, false));
        let rb = c.interact(behind, btn_rect());
        let rf = c.interact(front, btn_rect());
        assert!(!rb.hovered);
        assert!(rf.hovered);
        c.end();
    }

    #[test]
    fn drag_delta_reports_movement() {
        let mut c = ctx();
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), true, false));
        let r = c.interact(BTN, btn_rect());
        assert!(r.dragging);
        c.end();

        c.begin(frame_input(Some(Vec2::new(60.0, 25.0)), false, false));
        let r = c.interact(BTN, btn_rect());
        assert!(r.dragging);
        assert_eq!(r.drag_delta, Vec2::new(10.0, 10.0));
        c.end();
    }

    #[test]
    fn tab_cycles_focus() {
        let mut c = ctx();
        let a = Id::from(1u8);
        let b = Id::from(2u8);
        let base = |tab: bool| FrameInput {
            keys: if tab {
                vec![KeyEvent {
                    key: Key::Tab,
                    mods: Default::default(),
                    pressed: true,
                    repeat: false,
                }]
            } else {
                vec![]
            },
            ..Default::default()
        };

        c.begin(base(false));
        c.register_focusable(a);
        c.register_focusable(b);
        c.end();

        c.begin(base(true));
        c.register_focusable(a);
        c.register_focusable(b);
        c.end();
        assert_eq!(
            c.memory.focus,
            Some(a),
            "first Tab focuses the first focusable"
        );

        c.begin(base(true));
        c.register_focusable(a);
        c.register_focusable(b);
        c.end();
        assert_eq!(c.memory.focus, Some(b));

        // Shift+Tab goes back and wraps.
        let mut back = base(true);
        if let Some(k) = back.keys.first_mut() {
            k.mods.shift = true;
        }
        c.begin(back);
        c.register_focusable(a);
        c.register_focusable(b);
        c.end();
        assert_eq!(c.memory.focus, Some(a));
    }

    #[test]
    fn consumed_key_is_not_seen_twice() {
        let mut c = ctx();
        c.begin(FrameInput {
            keys: vec![KeyEvent {
                key: Key::Enter,
                mods: Default::default(),
                pressed: true,
                repeat: false,
            }],
            ..Default::default()
        });
        assert!(c.key_pressed(Key::Enter, false));
        assert!(!c.key_pressed(Key::Enter, false), "second ask must fail");
        c.end();
    }

    #[test]
    fn same_frame_click_registers() {
        let mut c = ctx();
        // Register the widget first.
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        // Press AND release arrive within one frame (fast click / event
        // coalescing): the widget must still report a click.
        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), false, false));
        c.input.primary_pressed = true;
        c.input.primary_released = true;
        c.input.primary_down = false;
        let r = c.interact(BTN, btn_rect());
        assert!(r.clicked, "a full click inside one frame must not be lost");
        assert!(!r.dragging);
        c.end();
        assert_eq!(
            c.memory.pointer_capture, None,
            "capture released with the pointer"
        );
    }

    #[test]
    fn press_release_press_keeps_dragging() {
        let mut c = ctx();
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        // Press, then release+re-press inside one frame: the drag continues.
        c.begin(frame_input(Some(Vec2::new(50.0, 15.0)), true, false));
        c.interact(BTN, btn_rect());
        c.end();

        c.begin(frame_input(Some(Vec2::new(60.0, 15.0)), false, false));
        c.input.primary_released = true;
        c.input.primary_pressed = true;
        c.input.primary_down = true;
        let r = c.interact(BTN, btn_rect());
        assert!(r.clicked, "the completed first click registers");
        assert!(r.dragging, "the second press continues the drag");
        c.end();
        assert_eq!(
            c.memory.pointer_capture,
            Some(BTN),
            "capture survives the re-press"
        );
    }

    fn click_at(c: &mut Ctx, pos: Vec2, time_ms: f64) -> Response {
        c.begin(frame_input(Some(pos), true, false));
        c.input.time_ms = time_ms;
        c.interact(BTN, btn_rect());
        c.end();
        c.begin(frame_input(Some(pos), false, true));
        c.input.time_ms = time_ms;
        let r = c.interact(BTN, btn_rect());
        c.end();
        r
    }

    #[test]
    fn double_click_within_400ms() {
        let mut c = ctx();
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        let pos = Vec2::new(50.0, 15.0);
        let first = click_at(&mut c, pos, 1_000.0);
        assert!(first.clicked);
        assert!(!first.double_clicked, "the first click has no pair");

        let second = click_at(&mut c, pos, 1_400.0);
        assert!(second.clicked);
        assert!(
            second.double_clicked,
            "400 ms and no movement is a double-click"
        );
    }

    #[test]
    fn double_click_rejected_after_600ms() {
        let mut c = ctx();
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        let pos = Vec2::new(50.0, 15.0);
        click_at(&mut c, pos, 1_000.0);
        let second = click_at(&mut c, pos, 1_600.0);
        assert!(second.clicked);
        assert!(
            !second.double_clicked,
            "600 ms is outside the 500 ms window"
        );
    }

    #[test]
    fn double_click_rejected_when_pointer_moves_20px() {
        let mut c = ctx();
        c.begin(frame_input(None, false, false));
        c.interact(BTN, btn_rect());
        c.end();

        click_at(&mut c, Vec2::new(50.0, 15.0), 1_000.0);
        let second = click_at(&mut c, Vec2::new(70.0, 15.0), 1_100.0);
        assert!(second.clicked);
        assert!(
            !second.double_clicked,
            "20 px of travel is outside the 6 px slop"
        );
    }
}
