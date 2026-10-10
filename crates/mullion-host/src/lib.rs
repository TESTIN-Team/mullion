//! Windows host backend for egui: Win32 window, per-monitor DPI, IME,
//! clipboard, and a GL 1.1 painter that draws egui's tessellated output
//! through a WGL compatibility context.

pub mod clipboard;
pub mod input_map;
pub mod paint;
pub mod png;
pub mod wgl;
pub mod window;

pub use window::{EguiApp, RunStats, SmokeConfig, WindowConfig, run_egui};
