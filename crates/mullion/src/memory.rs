//! Cross-frame widget memory: focus, pointer capture, scroll offsets and
//! text edit buffers. Everything here is a pure state machine and is fully
//! unit-testable without a window.

use crate::geometry::Vec2;
use crate::input::Dir;
use crate::text::TextLayout;
use std::collections::HashMap;

/// Stable widget id (hash of a debug string, deterministic per run and
/// across runs because `DefaultHasher` uses fixed keys).
pub type Id = u64;

pub fn id_of(s: &str) -> Id {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Single-line text edit state: buffer, caret and optional selection anchor.
/// Byte offsets are always on char boundaries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditState {
    pub buf: String,
    pub caret: usize,
    /// Selection anchor; selection spans `min(caret, anchor)..max(..)` when set.
    pub anchor: Option<usize>,
    /// Horizontal scroll offset in physical pixels, managed by the widget.
    pub scroll_x: f32,
}

impl EditState {
    pub fn new(text: &str) -> Self {
        Self {
            buf: text.to_string(),
            caret: text.len(),
            anchor: None,
            scroll_x: 0.0,
        }
    }

    pub fn text(&self) -> &str {
        &self.buf
    }

    /// Ordered selection range, if a selection exists.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.anchor.map(|a| {
            if a <= self.caret {
                (a, self.caret)
            } else {
                (self.caret, a)
            }
        })
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selection().map(|(s, e)| &self.buf[s..e])
    }

    fn clamp_boundary(&self, mut i: usize) -> usize {
        i = i.min(self.buf.len());
        while i > 0 && !self.buf.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    fn delete_range(&mut self, s: usize, e: usize) {
        self.buf.replace_range(s..e, "");
        self.caret = s;
        self.anchor = None;
    }

    /// Replace selection (or insert at caret) with `s`; newlines are dropped.
    pub fn insert(&mut self, s: &str) {
        let s: String = s.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        if s.is_empty() && self.selection().is_none() {
            return;
        }
        let (s0, e0) = self.selection().unwrap_or((self.caret, self.caret));
        let mut new = String::with_capacity(self.buf.len() + s.len());
        new.push_str(&self.buf[..s0]);
        new.push_str(&s);
        new.push_str(&self.buf[e0..]);
        let new_caret = s0 + s.len();
        self.buf = new;
        self.caret = new_caret;
        self.anchor = None;
    }

    /// Delete the selection, or the char before the caret.
    pub fn backspace(&mut self) {
        if let Some((s, e)) = self.selection() {
            self.delete_range(s, e);
            return;
        }
        if self.caret == 0 {
            return;
        }
        let mut s = self.caret - 1;
        while !self.buf.is_char_boundary(s) {
            s -= 1;
        }
        self.delete_range(s, self.caret);
    }

    /// Delete the selection, or the char after the caret.
    pub fn delete_forward(&mut self) {
        if let Some((s, e)) = self.selection() {
            self.delete_range(s, e);
            return;
        }
        if self.caret >= self.buf.len() {
            return;
        }
        let mut e = self.caret + 1;
        while e < self.buf.len() && !self.buf.is_char_boundary(e) {
            e += 1;
        }
        self.delete_range(self.caret, e);
    }

    fn neighbor(&self, from: usize, dir: Dir) -> usize {
        match dir {
            Dir::Left => {
                let mut i = from;
                while i > 0 {
                    i -= 1;
                    if self.buf.is_char_boundary(i) {
                        break;
                    }
                }
                i
            }
            Dir::Right => {
                let mut i = from + 1;
                while i < self.buf.len() && !self.buf.is_char_boundary(i) {
                    i += 1;
                }
                i.min(self.buf.len())
            }
            Dir::LineStart => 0,
            Dir::LineEnd => self.buf.len(),
        }
    }

    /// Move the caret; with `select` the current selection is extended
    /// (anchor stays, caret moves).
    pub fn move_caret(&mut self, dir: Dir, select: bool) {
        if !select {
            self.anchor = None;
        } else if self.anchor.is_none() {
            self.anchor = Some(self.caret);
        }
        self.caret = self.clamp_boundary(self.neighbor(self.caret, dir));
        if let Some(a) = self.anchor {
            if a == self.caret {
                self.anchor = None;
            }
        }
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.caret = self.buf.len();
    }

    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Remove and return the selection.
    pub fn cut(&mut self) -> Option<String> {
        let (s, e) = self.selection()?;
        let out = self.buf[s..e].to_string();
        self.delete_range(s, e);
        Some(out)
    }

    pub fn set_text(&mut self, s: &str) {
        self.buf = s.to_string();
        self.caret = self.buf.len();
        self.anchor = None;
        self.scroll_x = 0.0;
    }
}

/// Cross-frame widget state keyed by [`Id`].
#[derive(Clone, Debug, Default)]
pub struct Memory {
    pub focus: Option<Id>,
    /// Id capturing the pointer during a press/drag.
    pub pointer_capture: Option<Id>,
    /// Vertical scroll offsets per scroll area, logical pixels.
    pub scroll: HashMap<Id, f32>,
    /// Edit buffers per line edit.
    pub edit: HashMap<Id, EditState>,
    /// Pointer position last frame (logical px), for drag deltas.
    pub last_mouse: Option<Vec2>,
    /// Interaction rects registered last frame, in z order (later = on top).
    pub prev_rects: Vec<(Id, crate::geometry::Rect)>,
    /// Focusable ids registered last frame, in registration order.
    pub prev_focusables: Vec<Id>,
}

impl Memory {
    pub fn edit_state(&mut self, id: Id) -> &mut EditState {
        self.edit.entry(id).or_default()
    }

    pub fn scroll_y(&mut self, id: Id) -> &mut f32 {
        self.scroll.entry(id).or_insert(0.0)
    }
}

/// Iterate the selected byte ranges of an edit buffer as visual slices for
/// rendering. Returns up to one range in v0.1 (no column layouts).
pub fn selection_ranges(st: &EditState, _layout: &TextLayout) -> Vec<(usize, usize)> {
    st.selection().map(|r| vec![r]).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_replaces_selection() {
        let mut e = EditState::new("hello");
        e.select_all();
        e.insert("world");
        assert_eq!(e.text(), "world");
        assert_eq!(e.caret, 5);
        assert_eq!(e.selection(), None);
    }

    #[test]
    fn insert_drops_newlines() {
        let mut e = EditState::new("");
        e.insert("a\nb\r\nc");
        assert_eq!(e.text(), "abc");
    }

    #[test]
    fn backspace_char_and_multibyte() {
        let mut e = EditState::new("a中b");
        e.caret = 4; // after 中
        e.backspace();
        assert_eq!(e.text(), "ab");
        e.backspace();
        assert_eq!(e.text(), "b");
        // At start: no-op.
        e.caret = 0;
        e.backspace();
        assert_eq!(e.text(), "b");
    }

    #[test]
    fn delete_forward_works() {
        let mut e = EditState::new("ab");
        e.caret = 0;
        e.delete_forward();
        assert_eq!(e.text(), "b");
    }

    #[test]
    fn backspace_removes_selection() {
        let mut e = EditState::new("abcdef");
        e.caret = 2;
        e.move_caret(Dir::Right, true); // select "c"
        e.move_caret(Dir::Right, true); // select "cd"
        assert_eq!(e.selected_text(), Some("cd"));
        e.backspace();
        assert_eq!(e.text(), "abef");
        assert_eq!(e.caret, 2);
    }

    #[test]
    fn caret_movement_clamps_to_boundaries() {
        let mut e = EditState::new("a中b");
        e.caret = 1;
        // Walking left from the boundary before 中 lands before a.
        e.move_caret(Dir::Left, false);
        assert_eq!(e.caret, 0);
        e.move_caret(Dir::Left, false);
        assert_eq!(e.caret, 0, "clamped at start");
        e.move_caret(Dir::Right, false);
        assert_eq!(e.caret, 1);
        e.move_caret(Dir::Right, false);
        assert_eq!(e.caret, 4, "one Right skips the full 3-byte char");
        e.move_caret(Dir::Right, false);
        assert_eq!(e.caret, 5, "clamped at end");
        e.move_caret(Dir::LineStart, false);
        assert_eq!(e.caret, 0);
        e.move_caret(Dir::LineEnd, false);
        assert_eq!(e.caret, 5);
    }

    #[test]
    fn shift_selection_extends_then_collapses() {
        let mut e = EditState::new("abc");
        e.caret = 1;
        e.move_caret(Dir::Right, true);
        assert_eq!(e.selection(), Some((1, 2)));
        e.move_caret(Dir::Left, true);
        assert_eq!(e.selection(), None, "back at anchor collapses");
    }

    #[test]
    fn cut_removes_and_returns() {
        let mut e = EditState::new("hello world");
        e.caret = 5;
        e.move_caret(Dir::LineEnd, true);
        assert_eq!(e.cut(), Some(" world".to_string()));
        assert_eq!(e.text(), "hello");
        assert_eq!(e.cut(), None);
    }

    #[test]
    fn select_all_and_copy_slice() {
        let mut e = EditState::new("你好");
        e.select_all();
        assert_eq!(e.selected_text(), Some("你好"));
        assert_eq!(e.selection(), Some((0, 6)));
    }

    #[test]
    fn id_of_is_stable() {
        assert_eq!(id_of("gallery/name"), id_of("gallery/name"));
        assert_ne!(id_of("a"), id_of("b"));
    }
}
