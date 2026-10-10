//! Immediate-mode widgets. Every widget draws and resolves interaction for
//! one frame; state either lives in the caller (`on: bool`, slider value) or
//! in [`crate::memory::Memory`] keyed by an explicit id (line edits, scroll
//! areas, windows).
//!
//! All rect parameters are logical pixels.

use crate::color::Color;
use crate::context::Ctx;
use crate::geometry::{Rect, Vec2};
use crate::input::{Dir, Key};
use crate::memory::{Id, id_of};
use crate::text::TextLayout;
use std::sync::Arc;

const EDIT_PAD: f32 = 8.0;

/// Stable id for a sub-part of a widget (e.g. a window's close button).
fn sub_id(parent: Id, part: &str) -> Id {
    parent ^ id_of(part)
}

/// Position-derived id for stateless widgets (buttons, toggles).
fn auto_id(rect: Rect) -> Id {
    let s = format!(
        "auto/{:.4}/{:.4}/{:.4}/{:.4}",
        rect.min.x, rect.min.y, rect.max.x, rect.max.y
    );
    id_of(&s)
}

fn stroke(ctx: &Ctx, color: Color) -> crate::draw::Stroke {
    crate::draw::Stroke::new(color, ctx.style.spacing.border_w)
}

// ---- text helpers ----

/// Draw `text` horizontally and vertically centered in `rect`.
pub fn centered_text(ctx: &mut Ctx, rect: Rect, text: &str, color: Color) -> Arc<TextLayout> {
    let layout = ctx.layout_text(text);
    let s = ctx.style.scale;
    let x = rect.center().x - layout.width / (2.0 * s);
    let y = rect.center().y + (layout.ascent - layout.descent) / (2.0 * s);
    ctx.text_at(Arc::clone(&layout), Vec2::new(x, y), color);
    layout
}

/// Draw `text` left-aligned, vertically centered in `rect`.
pub fn label_text(ctx: &mut Ctx, rect: Rect, text: &str, color: Color) -> Arc<TextLayout> {
    let layout = ctx.layout_text(text);
    let y = rect.center().y + (layout.ascent - layout.descent) / (2.0 * ctx.style.scale);
    ctx.text_at(Arc::clone(&layout), Vec2::new(rect.min.x, y), color);
    layout
}

/// Hover hint. Appears after the pointer has stayed inside `rect` for
/// 800 ms of injected `time_ms`. Drawn on a new layer, above the pointer
/// and clamped to `FrameInput::screen` when the host provides one.
/// Leaving the rect or losing the pointer restarts the timer.
pub fn tooltip(ctx: &mut Ctx, id: Id, rect: Rect, text: &str) -> bool {
    ctx.queue_tooltip(id, rect, text)
}

/// A static text label, clipped to `rect`.
pub fn label(ctx: &mut Ctx, rect: Rect, text: &str, color: Option<Color>) {
    let color = color.unwrap_or(ctx.style.theme.text);
    ctx.push_clip(rect);
    label_text(ctx, rect, text, color);
    ctx.pop_clip();
}

// ---- buttons ----

/// A push button. `clicked` is also set for Enter while focused.
/// `double_clicked` stays pointer-only.
pub fn button(ctx: &mut Ctx, rect: Rect, text: &str) -> crate::context::Response {
    let id = auto_id(rect);
    let mut resp = ctx.interact(id, rect);
    ctx.register_focusable(id);

    let th = ctx.style.theme;
    let sp = ctx.style.spacing;
    let fill = if resp.dragging && resp.hovered {
        th.surface_active
    } else if resp.hovered {
        th.surface_hover
    } else {
        th.surface
    };
    ctx.fill_stroke(rect, Some(fill), Some(stroke(ctx, th.border)), sp.rounding);

    if resp.focused && ctx.key_pressed(Key::Enter, false) {
        resp.clicked = true;
    }
    centered_text(ctx, rect, text, th.text);
    if resp.focused {
        ctx.fill_stroke(rect, None, Some(stroke(ctx, th.accent)), sp.rounding);
    }
    resp
}

/// A pill toggle switch. Returns the new state.
pub fn toggle(ctx: &mut Ctx, rect: Rect, on: bool) -> bool {
    let id = auto_id(rect);
    let resp = ctx.interact(id, rect);
    ctx.register_focusable(id);

    let mut on = on;
    if resp.clicked || (resp.focused && ctx.key_pressed(Key::Enter, false)) {
        on = !on;
    }

    let th = ctx.style.theme;
    let h = rect.height().min(rect.width() * 0.5);
    let track = Rect::from_min_size(
        Vec2::new(rect.min.x, rect.center().y - h * 0.5),
        Vec2::new(rect.width(), h),
    );
    let (track_col, knob_col) = if on {
        (th.accent, th.on_accent)
    } else {
        (th.surface_active, th.text_dim)
    };
    ctx.fill_stroke(track, Some(track_col), None, h * 0.5);

    let knob_d = h - 4.0;
    let travel = track.width() - knob_d - 4.0;
    let knob_x = track.min.x + 2.0 + travel * if on { 1.0 } else { 0.0 };
    let knob = Rect::from_min_size(
        Vec2::new(knob_x, track.center().y - knob_d * 0.5),
        Vec2::new(knob_d, knob_d),
    );
    ctx.fill_stroke(knob, Some(knob_col), None, knob_d * 0.5);
    if resp.focused {
        ctx.fill_stroke(track, None, Some(stroke(ctx, th.accent)), h * 0.5);
    }
    on
}

/// A checkbox. Returns the new state.
pub fn checkbox(ctx: &mut Ctx, rect: Rect, on: bool) -> bool {
    let id = auto_id(rect);
    let resp = ctx.interact(id, rect);
    ctx.register_focusable(id);

    let mut on = on;
    if resp.clicked || (resp.focused && ctx.key_pressed(Key::Enter, false)) {
        on = !on;
    }

    let th = ctx.style.theme;
    let side = rect.height().min(20.0);
    let box_r = Rect::from_min_size(
        Vec2::new(rect.min.x, rect.center().y - side * 0.5),
        Vec2::new(side, side),
    );
    let fill = if on {
        th.accent
    } else if resp.hovered {
        th.surface_hover
    } else {
        th.surface
    };
    ctx.fill_stroke(box_r, Some(fill), Some(stroke(ctx, th.border)), 3.0);
    if on {
        let c = box_r.center();
        let p1 = c.sub(Vec2::new(side * 0.22, side * 0.02));
        let p2 = c.add(Vec2::new(side * 0.10, side * 0.18));
        let p3 = c.add(Vec2::new(side * 0.24, -side * 0.20));
        ctx.line(p1, p2, th.on_accent, 2.0);
        ctx.line(p2, p3, th.on_accent, 2.0);
    }
    if resp.focused {
        ctx.fill_stroke(box_r, None, Some(stroke(ctx, th.accent)), 3.0);
    }
    on
}

/// A horizontal slider. Returns the new value (clamped to `[min, max]`).
pub fn slider(ctx: &mut Ctx, rect: Rect, id: Id, value: f32, min: f32, max: f32) -> f32 {
    let span = (max - min).abs().max(f32::EPSILON);
    let mut norm = ((value - min) / span).clamp(0.0, 1.0);
    let resp = ctx.interact(id, rect);
    ctx.register_focusable(id);

    if resp.dragging {
        if let Some(m) = ctx.input.mouse_pos {
            norm = ((m.x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0);
        }
    }
    if resp.focused {
        let step = 1.0 / 20.0;
        if ctx.key_pressed(Key::Left, false) {
            norm = (norm - step).clamp(0.0, 1.0);
        }
        if ctx.key_pressed(Key::Right, false) {
            norm = (norm + step).clamp(0.0, 1.0);
        }
    }

    let th = ctx.style.theme;
    let track_h = 6.0;
    let track = Rect::from_min_size(
        Vec2::new(rect.min.x, rect.center().y - track_h * 0.5),
        Vec2::new(rect.width(), track_h),
    );
    let filled = Rect::from_min_size(track.min, Vec2::new(track.width() * norm, track.height()));
    ctx.fill_stroke(track, Some(th.surface_active), None, track_h * 0.5);
    ctx.fill_stroke(filled, Some(th.accent), None, track_h * 0.5);

    let knob_d = 14.0;
    let knob_x = track.min.x + (track.width() - knob_d) * norm;
    let knob = Rect::from_min_size(
        Vec2::new(knob_x, track.center().y - knob_d * 0.5),
        Vec2::new(knob_d, knob_d),
    );
    let knob_col = if resp.hovered || resp.dragging {
        th.accent_hover
    } else {
        th.accent
    };
    ctx.fill_stroke(knob, Some(knob_col), None, knob_d * 0.5);
    if resp.focused {
        ctx.fill_stroke(rect.inflate(2.0), None, Some(stroke(ctx, th.accent)), 4.0);
    }

    min + norm * span
}

/// A progress bar; `frac` is clamped to `[0, 1]`.
pub fn progress(ctx: &mut Ctx, rect: Rect, frac: f32) {
    let th = ctx.style.theme;
    let frac = frac.clamp(0.0, 1.0);
    let h = 8.0;
    let track = Rect::from_min_size(
        Vec2::new(rect.min.x, rect.center().y - h * 0.5),
        Vec2::new(rect.width(), h),
    );
    ctx.fill_stroke(track, Some(th.surface_active), None, h * 0.5);
    let filled = Rect::from_min_size(track.min, Vec2::new(track.width() * frac, h));
    ctx.fill_stroke(filled, Some(th.accent), None, h * 0.5);
}

/// A full-width 1px horizontal separator centered in `rect`.
pub fn separator(ctx: &mut Ctx, rect: Rect) {
    let y = rect.center().y;
    ctx.line(
        Vec2::new(rect.min.x, y),
        Vec2::new(rect.max.x, y),
        ctx.style.theme.border,
        1.0,
    );
}

// ---- text edit ----

/// Set the caret from a logical-space pointer position inside an edit rect.
/// `extend` turns the move into selection extension.
fn place_caret(ctx: &mut Ctx, id: Id, rect: Rect, m: Vec2, extend: bool) {
    let scale = ctx.style.scale;
    let scroll = ctx.memory.edit_state(id).scroll_x;
    let buf = ctx.memory.edit_state(id).buf.clone();
    let layout = ctx.layout_text(&buf);
    let local_phys = (m.x - rect.min.x - EDIT_PAD) * scale + scroll;
    let byte = ctx.shaper.hit(&layout, local_phys);
    let st = ctx.memory.edit_state(id);
    if extend {
        if st.anchor.is_none() {
            st.anchor = Some(st.caret);
        }
    } else {
        st.anchor = None;
    }
    st.caret = byte;
}

/// Single-line text edit with selection, clipboard shortcuts and IME
/// composition display. Returns the current buffer content.
pub fn line_edit(ctx: &mut Ctx, rect: Rect, id: Id, placeholder: &str) -> String {
    let resp = ctx.interact(id, rect);
    ctx.register_focusable(id);
    let focused = resp.focused;
    let th = ctx.style.theme;
    let sp = ctx.style.spacing;
    let scale = ctx.style.scale;

    // Click positions the caret; press-drag extends the selection.
    if let Some(m) = ctx.input.mouse_pos {
        if resp.drag_started || resp.clicked {
            ctx.memory.focus = Some(id);
            place_caret(ctx, id, rect, m, false);
        } else if resp.dragging && focused {
            place_caret(ctx, id, rect, m, true);
        }
    }

    if focused {
        if ctx.key_pressed(Key::Backspace, false) {
            ctx.memory.edit_state(id).backspace();
        }
        if ctx.key_pressed(Key::Delete, false) {
            ctx.memory.edit_state(id).delete_forward();
        }
        if ctx.key_pressed(Key::A, true) {
            ctx.memory.edit_state(id).select_all();
        }
        if ctx.key_pressed(Key::C, true) {
            if let Some(t) = ctx
                .memory
                .edit_state(id)
                .selected_text()
                .map(str::to_string)
            {
                ctx.request_copy(t);
            }
        }
        if ctx.key_pressed(Key::X, true) {
            if let Some(t) = ctx.memory.edit_state(id).cut() {
                ctx.request_copy(t);
            }
        }
        if ctx.key_pressed(Key::V, true) {
            ctx.request_paste(id);
        }
        while let Some(ev) = ctx.take_key(|ev| {
            ev.pressed
                && !ev.mods.ctrl
                && !ev.mods.alt
                && matches!(ev.key, Key::Left | Key::Right | Key::Home | Key::End)
        }) {
            let dir = match ev.key {
                Key::Left => Dir::Left,
                Key::Right => Dir::Right,
                Key::Home => Dir::LineStart,
                _ => Dir::LineEnd,
            };
            ctx.memory.edit_state(id).move_caret(dir, ev.mods.shift);
        }
        if ctx.key_pressed(Key::Escape, false) {
            ctx.memory.focus = None;
        }
        if let Some(text) = ctx.input.paste.clone() {
            ctx.memory.edit_state(id).insert(&text);
        }
        if !ctx.input.text.is_empty() && ctx.input.preedit.is_none() {
            let t = ctx.input.text.clone();
            ctx.memory.edit_state(id).insert(&t);
        }
    }

    let buf = ctx.memory.edit_state(id).buf.clone();
    let text_layout = if buf.is_empty() {
        None
    } else {
        Some(ctx.layout_text(&buf))
    };

    // Caret x in physical px, relative to the pen.
    let caret_x = match &text_layout {
        Some(l) => ctx.shaper.caret_x(l, ctx.memory.edit_state(id).caret),
        None => 0.0,
    };

    // Keep the caret visible by adjusting the horizontal scroll.
    let view_w = ((rect.width() - EDIT_PAD * 2.0) * scale).max(1.0);
    {
        let st = ctx.memory.edit_state(id);
        if caret_x < st.scroll_x {
            st.scroll_x = caret_x;
        } else if caret_x > st.scroll_x + view_w {
            st.scroll_x = caret_x - view_w;
        }
        st.scroll_x = st.scroll_x.max(0.0);
    }
    let scroll_x = ctx.memory.edit_state(id).scroll_x;

    // ---- drawing ----
    let border = if focused { th.accent } else { th.border };
    ctx.fill_stroke(
        rect,
        Some(th.surface),
        Some(stroke(ctx, border)),
        sp.rounding,
    );

    let inner = Rect::from_min_size(
        Vec2::new(rect.min.x + EDIT_PAD, rect.min.y),
        Vec2::new((rect.width() - EDIT_PAD * 2.0).max(0.0), rect.height()),
    );
    ctx.push_clip(inner);
    let pen = inner.min.x - scroll_x / scale;

    let has_preedit = focused && ctx.input.preedit.is_some();
    if let Some(l) = &text_layout {
        let baseline = inner.center().y + (l.ascent - l.descent) / (2.0 * scale);
        if let Some((s, e)) = ctx.memory.edit_state(id).selection() {
            let x0 = ctx.shaper.caret_x(l, s);
            let x1 = ctx.shaper.caret_x(l, e);
            let sel_r = Rect::from_min_size(
                Vec2::new(pen + x0 / scale, inner.min.y + 2.0),
                Vec2::new(((x1 - x0) / scale).max(1.0), inner.height() - 4.0),
            );
            ctx.fill(sel_r, th.selection);
        }
        ctx.text_at(Arc::clone(l), Vec2::new(pen, baseline), th.text);

        if focused && !has_preedit {
            let cx = pen + caret_x / scale;
            let caret_r = Rect::from_min_size(
                Vec2::new(cx, inner.min.y + 3.0),
                Vec2::new(1.5, inner.height() - 6.0),
            );
            ctx.fill(caret_r, th.accent);
            ctx.set_ime_caret(Vec2::new(cx, baseline - l.ascent / scale));
        }
    } else if focused && !has_preedit {
        let caret_r = Rect::from_min_size(
            Vec2::new(pen, inner.min.y + 3.0),
            Vec2::new(1.5, inner.height() - 6.0),
        );
        ctx.fill(caret_r, th.accent);
        ctx.set_ime_caret(Vec2::new(pen, inner.min.y + 3.0));
    }

    // IME composition drawn at the caret with an underline.
    if let Some(preedit) = ctx.input.preedit.clone() {
        if focused {
            let pl = ctx.layout_text(&preedit.text);
            let baseline = inner.center().y + (pl.ascent - pl.descent) / (2.0 * scale);
            let pre_x = pen + caret_x / scale;
            ctx.text_at(Arc::clone(&pl), Vec2::new(pre_x, baseline), th.text);
            let uw_y = baseline + pl.descent / scale + 2.0;
            ctx.line(
                Vec2::new(pre_x, uw_y),
                Vec2::new(pre_x + pl.width / scale, uw_y),
                th.accent,
                1.5,
            );
            let cbyte = preedit.caret_byte.min(preedit.text.len());
            let cx = pre_x + ctx.shaper.caret_x(&pl, cbyte) / scale;
            let caret_r = Rect::from_min_size(
                Vec2::new(cx, inner.min.y + 3.0),
                Vec2::new(1.5, inner.height() - 6.0),
            );
            ctx.fill(caret_r, th.accent);
            ctx.set_ime_caret(Vec2::new(cx, baseline - pl.ascent / scale));
        }
    }
    ctx.pop_clip();

    // Placeholder when empty and unfocused.
    if text_layout.is_none() && !focused && !placeholder.is_empty() {
        let pl = ctx.layout_text(placeholder);
        let baseline = inner.center().y + (pl.ascent - pl.descent) / (2.0 * scale);
        // Re-clip because pop_clip closed the previous scope.
        ctx.push_clip(inner);
        ctx.text_at(
            Arc::clone(&pl),
            Vec2::new(inner.min.x, baseline),
            th.text_dim,
        );
        ctx.pop_clip();
    }

    ctx.memory.edit_state(id).buf.clone()
}

// ---- containers ----

/// A floating window with title bar, drag-to-move and a close button.
/// Returns the body rect while `*open` is true.
pub fn window(
    ctx: &mut Ctx,
    id: Id,
    title: &str,
    pos: &mut Vec2,
    size: Vec2,
    open: &mut bool,
) -> Option<Rect> {
    if !*open {
        return None;
    }
    let sp = ctx.style.spacing;
    let th = ctx.style.theme;

    let rect = Rect::from_min_size(*pos, size);
    let title_rect = Rect::from_min_size(rect.min, Vec2::new(size.x, sp.title_h));

    let resp = ctx.interact(id, title_rect);
    if resp.dragging {
        *pos = Vec2::new(pos.x + resp.drag_delta.x, pos.y + resp.drag_delta.y);
    }

    let close_size = 16.0;
    let close = Rect::from_min_size(
        Vec2::new(
            rect.max.x - sp.pad - close_size,
            title_rect.center().y - close_size * 0.5,
        ),
        Vec2::new(close_size, close_size),
    );
    let close_resp = ctx.interact(sub_id(id, "close"), close);
    if close_resp.clicked {
        *open = false;
        return None;
    }

    ctx.push_layer();
    let moved = Rect::from_min_size(*pos, size);
    ctx.fill_stroke(
        moved,
        Some(th.surface),
        Some(stroke(ctx, th.border)),
        sp.rounding,
    );
    ctx.fill_stroke(
        Rect::from_min_size(moved.min, Vec2::new(size.x, sp.title_h)),
        Some(th.surface_active),
        None,
        sp.rounding,
    );
    let label_r = Rect::from_min_size(
        moved.min.add(Vec2::new(sp.pad, 0.0)),
        Vec2::new(size.x - sp.pad * 2.0 - close_size, sp.title_h),
    );
    label_text(ctx, label_r, title, th.text);

    // ✕ drawn as two crisp lines.
    if close_resp.hovered {
        let cx = close.translate(moved.min.sub(rect.min)).center();
        let r = Rect::from_min_size(cx.sub(Vec2::new(10.0, 10.0)), Vec2::new(20.0, 20.0));
        ctx.fill_stroke(r, Some(th.surface_hover), None, 4.0);
    }
    let cx = close.translate(moved.min.sub(rect.min)).center();
    let a = cx.sub(Vec2::new(4.0, 4.0));
    let b = cx.add(Vec2::new(4.0, 4.0));
    ctx.line(a, b, th.text_dim, 1.5);
    ctx.line(Vec2::new(a.x, b.y), Vec2::new(b.x, a.y), th.text_dim, 1.5);

    let body = Rect::from_min_size(
        moved.min.add(Vec2::new(sp.pad, sp.title_h + sp.pad * 0.5)),
        Vec2::new(
            size.x - sp.pad * 2.0,
            (size.y - sp.title_h - sp.pad * 1.5).max(0.0),
        ),
    );
    Some(body)
}

/// A vertical scroll area. Calls `f(ctx, visible_content_rect)` with a clip
/// active; `content_h` is the unclipped logical height of the content. The
/// rect passed to `f` starts at the top of the (scrolled) content.
pub fn scroll_area(
    ctx: &mut Ctx,
    id: Id,
    body: Rect,
    content_h: f32,
    f: impl FnOnce(&mut Ctx, Rect),
) {
    let th = ctx.style.theme;
    let sp = ctx.style.spacing;

    let resp = ctx.interact(id, body);
    let max_scroll = (content_h - body.height()).max(0.0);
    let scroll = {
        let s = ctx.memory.scroll_y(id);
        if resp.hovered {
            *s += ctx.input.wheel;
        }
        *s = s.clamp(0.0, max_scroll);
        *s
    };

    let show_bar = content_h > body.height() + 0.5;
    let view = if show_bar {
        Rect::from_min_size(
            body.min,
            Vec2::new(body.width() - sp.scrollbar_w, body.height()),
        )
    } else {
        body
    };

    if show_bar {
        let frac = if max_scroll > 0.0 {
            scroll / max_scroll
        } else {
            0.0
        };
        let min_thumb = 24.0_f32.min(body.height());
        let thumb_h = (body.height() * body.height() / content_h).clamp(min_thumb, body.height());
        let travel = (body.height() - thumb_h).max(0.0);
        let thumb = Rect::from_min_size(
            Vec2::new(
                body.max.x - sp.scrollbar_w + 1.0,
                body.min.y + travel * frac,
            ),
            Vec2::new(sp.scrollbar_w - 2.0, thumb_h),
        );
        ctx.fill_stroke(thumb, Some(th.scrollbar), None, 3.0);
    }

    ctx.push_clip(view);
    let content = Rect::from_min_size(
        Vec2::new(view.min.x, view.min.y - scroll),
        Vec2::new(view.width(), content_h),
    );
    f(ctx, content);
    ctx.pop_clip();
}

/// Read the current scroll offset of a scroll area (logical px).
pub fn scroll_offset_y(ctx: &Ctx, id: Id) -> f32 {
    ctx.memory.scroll.get(&id).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{FrameInput, KeyEvent, Modifiers};
    use std::sync::Arc;

    struct FakeFont;
    impl crate::text::FontBackend for FakeFont {
        fn metrics(&self, _px: f32) -> crate::text::FontMetrics {
            crate::text::FontMetrics {
                ascent: 11.0,
                descent: 3.0,
                line_gap: 0.0,
            }
        }
        fn glyph(&self, ch: char, px: f32) -> Option<crate::text::Glyph> {
            Some(crate::text::Glyph {
                ch,
                px,
                width: 5,
                height: 10,
                left: 0,
                top: -8,
                advance: 6.0,
                alpha: vec![255; 50],
            })
        }
    }

    fn ctx() -> Ctx {
        Ctx::new(Arc::new(FakeFont))
    }

    const ED: Id = 7;
    fn edit_rect() -> Rect {
        Rect::from_xywh(0.0, 0.0, 200.0, 28.0)
    }

    fn input(mouse: Option<Vec2>, pressed: bool, released: bool) -> FrameInput {
        FrameInput {
            mouse_pos: mouse,
            primary_pressed: pressed,
            primary_released: released,
            primary_down: pressed || (!released && mouse.is_some()),
            ..Default::default()
        }
    }

    const TIP: Id = 99;

    fn tip_rect() -> Rect {
        Rect::from_xywh(0.0, 0.0, 100.0, 40.0)
    }

    fn hover(c: &mut Ctx, pos: Option<Vec2>, time_ms: f64) -> bool {
        c.begin(FrameInput {
            mouse_pos: pos,
            time_ms,
            ..Default::default()
        });
        let shown = tooltip(c, TIP, tip_rect(), "hint");
        c.end();
        shown
    }

    #[test]
    fn tooltip_shows_after_800ms() {
        let mut c = ctx();
        let inside = Some(Vec2::new(10.0, 10.0));
        assert!(
            !hover(&mut c, inside, 0.0),
            "the first sample only starts the timer"
        );
        assert!(
            hover(&mut c, Some(Vec2::new(80.0, 30.0)), 800.0),
            "800 ms inside the rect shows the tip, even if the pointer moved"
        );
    }

    #[test]
    fn tooltip_hidden_before_800ms() {
        let mut c = ctx();
        let inside = Some(Vec2::new(10.0, 10.0));
        assert!(!hover(&mut c, inside, 0.0));
        assert!(
            !hover(&mut c, inside, 600.0),
            "600 ms is still short of the delay"
        );
    }

    #[test]
    fn tooltip_resets_when_pointer_leaves() {
        let mut c = ctx();
        let inside = Some(Vec2::new(10.0, 10.0));
        assert!(!hover(&mut c, inside, 0.0));
        assert!(!hover(&mut c, inside, 500.0));
        assert!(!hover(&mut c, Some(Vec2::new(400.0, 400.0)), 700.0));
        assert!(!hover(&mut c, inside, 700.0), "re-entry starts a new timer");
        assert!(
            !hover(&mut c, inside, 1_300.0),
            "600 ms after re-entry is not enough"
        );
        assert!(
            hover(&mut c, inside, 1_500.0),
            "800 ms after re-entry shows it"
        );
    }

    #[test]
    fn tooltip_bubble_stays_inside_screen() {
        let mut c = ctx();
        let screen = Rect::from_xywh(0.0, 0.0, 100.0, 80.0);
        let pos = Some(Vec2::new(90.0, 4.0));
        c.begin(FrameInput {
            mouse_pos: pos,
            time_ms: 0.0,
            screen: Some(screen),
            ..Default::default()
        });
        assert!(!tooltip(&mut c, TIP, tip_rect(), "hint"));
        c.end();

        c.begin(FrameInput {
            mouse_pos: pos,
            time_ms: 800.0,
            screen: Some(screen),
            ..Default::default()
        });
        assert!(tooltip(&mut c, TIP, tip_rect(), "hint"));
        c.end();

        let bubble = c
            .display
            .layers
            .iter()
            .rev()
            .find_map(|layer| {
                layer.cmds.iter().find_map(|cmd| match cmd {
                    crate::draw::Cmd::Rect { rect, fill, .. } => fill.is_some().then_some(*rect),
                    _ => None,
                })
            })
            .expect("tooltip layer has a filled rect");
        assert!(bubble.min.x >= screen.min.x - 0.01);
        assert!(bubble.min.y >= screen.min.y - 0.01);
        assert!(bubble.max.x <= screen.max.x + 0.01);
        assert!(bubble.max.y <= screen.max.y + 0.01);
    }

    fn key(k: Key, ctrl: bool) -> KeyEvent {
        KeyEvent {
            key: k,
            mods: Modifiers {
                ctrl,
                ..Default::default()
            },
            pressed: true,
            repeat: false,
        }
    }

    #[test]
    fn line_edit_types_and_edits() {
        let mut c = ctx();
        c.begin(input(None, false, false));
        line_edit(&mut c, edit_rect(), ED, "占位");
        c.end();

        // Focus by clicking.
        c.begin(input(Some(Vec2::new(10.0, 14.0)), true, false));
        line_edit(&mut c, edit_rect(), ED, "占位");
        c.end();
        assert_eq!(c.memory.focus, Some(ED));

        // Type text.
        let mut fi = input(None, false, false);
        fi.text = "hi".into();
        c.begin(fi);
        assert_eq!(line_edit(&mut c, edit_rect(), ED, "占位"), "hi");
        c.end();

        // Backspace deletes one char.
        let mut fi = input(None, false, false);
        fi.keys = vec![key(Key::Backspace, false)];
        c.begin(fi);
        assert_eq!(line_edit(&mut c, edit_rect(), ED, "占位"), "h");
        c.end();

        // Escape clears focus.
        let mut fi = input(None, false, false);
        fi.keys = vec![key(Key::Escape, false)];
        c.begin(fi);
        line_edit(&mut c, edit_rect(), ED, "占位");
        c.end();
        assert_eq!(c.memory.focus, None);
    }

    #[test]
    fn line_edit_select_all_copy_and_paste() {
        let mut c = ctx();
        // Register and focus.
        c.begin(input(None, false, false));
        line_edit(&mut c, edit_rect(), ED, "");
        c.end();
        c.begin(input(Some(Vec2::new(10.0, 14.0)), true, false));
        line_edit(&mut c, edit_rect(), ED, "");
        c.end();

        let mut fi = input(None, false, false);
        fi.text = "abcd".into();
        c.begin(fi);
        line_edit(&mut c, edit_rect(), ED, "");
        c.end();

        // Ctrl+A then Ctrl+C must surface a copy request for "abcd".
        let mut fi = input(None, false, false);
        fi.keys = vec![key(Key::A, true), key(Key::C, true)];
        c.begin(fi);
        line_edit(&mut c, edit_rect(), ED, "");
        let out = c.end();
        assert_eq!(out.copy_text.as_deref(), Some("abcd"));
        assert_eq!(c.memory.edit_state(ED).selection(), Some((0, 4)));

        // Ctrl+V with host-delivered text replaces the selection.
        let mut fi = input(None, false, false);
        fi.paste = Some("xy".into());
        c.begin(fi);
        assert_eq!(line_edit(&mut c, edit_rect(), ED, ""), "xy");
        c.end();
    }

    #[test]
    fn toggle_flips_on_click() {
        let mut c = ctx();
        let rect = Rect::from_xywh(0.0, 0.0, 40.0, 20.0);
        c.begin(input(None, false, false));
        toggle(&mut c, rect, false);
        c.end();

        c.begin(input(Some(rect.center()), true, false));
        toggle(&mut c, rect, false);
        c.end();
        c.begin(input(Some(rect.center()), false, true));
        let on = toggle(&mut c, rect, false);
        c.end();
        assert!(on, "click must flip the toggle");
    }

    #[test]
    fn window_drag_moves_position() {
        let mut c = ctx();
        let mut pos = Vec2::new(10.0, 10.0);
        let mut open = true;
        let size = Vec2::new(300.0, 200.0);

        c.begin(input(None, false, false));
        window(&mut c, 1, "w", &mut pos, size, &mut open);
        c.end();

        // Press on the title bar, then move while held.
        c.begin(input(Some(Vec2::new(150.0, 20.0)), true, false));
        window(&mut c, 1, "w", &mut pos, size, &mut open);
        c.end();
        assert_eq!(pos, Vec2::new(10.0, 10.0), "no move on the press frame");

        c.begin(input(Some(Vec2::new(180.0, 50.0)), false, false));
        window(&mut c, 1, "w", &mut pos, size, &mut open);
        c.end();
        assert_eq!(pos, Vec2::new(40.0, 40.0));

        c.begin(input(Some(Vec2::new(190.0, 60.0)), false, true));
        window(&mut c, 1, "w", &mut pos, size, &mut open);
        c.end();
        assert_eq!(pos, Vec2::new(40.0, 40.0), "release keeps position");
    }

    #[test]
    fn window_close_button_closes() {
        let mut c = ctx();
        let mut pos = Vec2::new(0.0, 0.0);
        let mut open = true;
        let size = Vec2::new(300.0, 200.0);

        c.begin(input(None, false, false));
        window(&mut c, 2, "w", &mut pos, size, &mut open);
        c.end();

        // Press + release on the close button (top-right corner area).
        let pad = c.style.spacing.pad;
        let close_center = Vec2::new(300.0 - pad - 8.0, 16.0);
        c.begin(input(Some(close_center), true, false));
        window(&mut c, 2, "w", &mut pos, size, &mut open);
        c.end();
        c.begin(input(Some(close_center), false, true));
        let body = window(&mut c, 2, "w", &mut pos, size, &mut open);
        c.end();
        assert!(!open);
        assert!(body.is_none());
    }

    #[test]
    fn scroll_area_clamps_and_reports_offset() {
        let mut c = ctx();
        let body = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        c.begin(input(Some(Vec2::new(50.0, 25.0)), false, false));
        scroll_area(&mut c, 9, body, 500.0, |_ctx, content| {
            assert_eq!(content.height(), 500.0);
        });
        c.end();

        let mut fi = input(Some(Vec2::new(50.0, 25.0)), false, false);
        fi.wheel = 30.0;
        c.begin(fi);
        scroll_area(&mut c, 9, body, 500.0, |_, _| {});
        c.end();
        assert!((scroll_offset_y(&c, 9) - 30.0).abs() < 1e-5);

        let mut fi = input(Some(Vec2::new(50.0, 25.0)), false, false);
        fi.wheel = 5000.0;
        c.begin(fi);
        scroll_area(&mut c, 9, body, 500.0, |_, _| {});
        c.end();
        assert!((scroll_offset_y(&c, 9) - 450.0).abs() < 1e-5);
    }
}
