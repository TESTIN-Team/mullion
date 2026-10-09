//! # mullion
//!
//! A small, dependency-free immediate-mode GUI core: layout, widgets,
//! input state machines and a deterministic CPU rasterizer, all unit
//! testable without a window. Host backends (see `mullion-host`) provide
//! the window, font rasterization and presentation.
//!
//! Typical frame:
//!
//! ```ignore
//! ctx.begin(frame_input);
//! widgets::window(ctx, id, "Title", &mut pos, size, &mut open)
//!     .map(|body| { /* draw into the body */ });
//! let output = ctx.end();
//! render(&ctx.display, &ctx.shaper, &mut framebuffer);
//! ```
//!
//! All coordinates are logical pixels; `Style::scale` converts to physical
//! pixels at draw time.

pub mod color;
pub mod context;
pub mod draw;
pub mod geometry;
pub mod input;
pub mod layout;
pub mod memory;
pub mod render;
pub mod style;
pub mod text;
pub mod widgets;

pub use color::{Color, over};
pub use context::{Ctx, FrameOutput, Response};
pub use draw::{Cmd, Display, DrawList, Stroke};
pub use geometry::{Rect, Vec2};
pub use input::{Dir, FrameInput, Key, KeyEvent, Modifiers, Preedit};
pub use memory::{EditState, Id, Memory};
pub use render::{Framebuffer, render};
pub use style::{Spacing, Style, Theme};
pub use text::{FontBackend, FontMetrics, Glyph, Shaper, TextLayout};
