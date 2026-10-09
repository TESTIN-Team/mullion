//! Windows backend for the [`mullion`] GUI core.
//!
//! Owns a Win32 window with a WGL compatibility context, presents the
//! core's CPU-rasterized framebuffer as a textured quad, rasterizes glyphs
//! through GDI `GetGlyphOutlineW` with per-family fallback, and wires up
//! per-monitor DPI, IME and the clipboard.

pub mod clipboard;
pub mod gdi_font;
pub mod input_map;
pub mod png;
pub mod wgl;
pub mod window;

pub use gdi_font::GdiFontSet;
pub use window::{App, RunStats, SmokeConfig, WindowConfig, run};
