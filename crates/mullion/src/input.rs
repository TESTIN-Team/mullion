//! Per-frame input events. The host translates OS messages into logical
//! pixels and semantic keys; the core never sees scan codes or window rects.

use crate::geometry::Vec2;

/// Keyboard modifiers held during an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    pub const NONE: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: false,
    };
}

/// Semantic keys the core cares about. Letters exist only for shortcuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Backspace,
    Delete,
    Enter,
    Escape,
    Tab,
    A,
    C,
    V,
    X,
}

/// One key transition this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub mods: Modifiers,
    pub pressed: bool,
    pub repeat: bool,
}

/// Caret movement direction for text fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    LineStart,
    LineEnd,
}

/// Active IME composition shown inside a text field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preedit {
    pub text: String,
    /// Byte offset of the composition caret inside `text` (0 = start).
    pub caret_byte: usize,
}

/// Everything that happened since the previous frame.
#[derive(Clone, Debug, Default)]
pub struct FrameInput {
    /// Primary (left) pointer button.
    pub primary_down: bool,
    pub primary_pressed: bool,
    pub primary_released: bool,
    /// Secondary (right) pointer button state (down only in v0.1).
    pub secondary_down: bool,
    /// Pointer position in logical pixels; `None` when it left the window.
    pub mouse_pos: Option<Vec2>,
    /// Wheel delta in logical pixels, positive = scroll content down.
    pub wheel: f32,
    pub keys: Vec<KeyEvent>,
    /// Committed text input (IME results and ordinary characters).
    pub text: String,
    /// Active IME composition, if any.
    pub preedit: Option<Preedit>,
    /// Clipboard text delivered by the host for the focused edit field.
    pub paste: Option<String>,
    /// Monotonic milliseconds from a host-chosen epoch. The core never
    /// reads a clock; tests and the host both supply this.
    pub time_ms: f64,
}
