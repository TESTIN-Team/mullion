//! Win32 window + run loop for mullion apps: message pump, per-monitor DPI,
//! IME composition, clipboard glue, vsync presentation, and a headless
//! `--smoke` mode that renders a fixed number of frames and writes a PNG
//! receipt.

use crate::clipboard;
use crate::gdi_font::GdiFontSet;
use crate::input_map::{hi_short, lo_short, vk_to_key};
use crate::png;
use crate::wgl::WglPresent;
use mullion::input::{FrameInput, KeyEvent, Modifiers, Preedit};
use mullion::render::{Framebuffer, render};
use mullion::{Ctx, Vec2};
use std::time::Instant;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC, ScreenToClient, ValidateRect};
use windows_sys::Win32::Graphics::OpenGL::{
    ChoosePixelFormat, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_MAIN_PLANE, PFD_STEREO_DONTCARE,
    PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR, SetPixelFormat,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture, SetCapture};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRect, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DestroyWindow, DispatchMessageW, GetClientRect, IDC_ARROW, LoadCursorW, MSG, PM_REMOVE,
    PeekMessageW, PostQuitMessage, RegisterClassW, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOZORDER, SetWindowPos, ShowWindow, TranslateMessage, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    WS_VISIBLE,
};

// ---- local system declarations (IME; the documented system-API exception) ----

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
type HIMC = *mut core::ffi::c_void;

#[repr(C)]
#[allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]
struct COMPOSITIONFORM {
    dwStyle: u32,
    ptCurrentPos: POINT,
    rcArea: RECT,
}

#[repr(C)]
#[allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]
struct CANDIDATEFORM {
    dwIndex: u32,
    dwStyle: u32,
    ptCurrentPos: POINT,
    rcArea: RECT,
}

// imm32 is not shipped as an import library with the MinGW toolchain, so
// the IMM entry points are resolved at runtime via GetProcAddress.
struct ImeApi {
    get_context: Option<unsafe extern "system" fn(HWND) -> HIMC>,
    release_context: Option<unsafe extern "system" fn(HWND, HIMC) -> i32>,
    get_composition_string_w:
        Option<unsafe extern "system" fn(HIMC, u32, *mut core::ffi::c_void, u32) -> i32>,
    set_composition_window: Option<unsafe extern "system" fn(HIMC, *const COMPOSITIONFORM) -> i32>,
    set_candidate_window: Option<unsafe extern "system" fn(HIMC, *const CANDIDATEFORM) -> i32>,
}

unsafe fn load_proc<T>(h: windows_sys::Win32::Foundation::HMODULE, name: &[u8]) -> Option<T> {
    unsafe {
        // PROC is Option<unsafe extern "system" fn() -> isize>: `?` unwraps it.
        let p = windows_sys::Win32::System::LibraryLoader::GetProcAddress(h, name.as_ptr())?;
        // Callers only ever pass fn-pointer types of the same size.
        Some(std::mem::transmute_copy(&p))
    }
}

fn ime_api() -> &'static ImeApi {
    static API: std::sync::OnceLock<ImeApi> = std::sync::OnceLock::new();
    API.get_or_init(|| unsafe {
        let name: Vec<u16> = "imm32.dll\0".encode_utf16().collect();
        let h = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(name.as_ptr());
        if h.is_null() {
            return ImeApi {
                get_context: None,
                release_context: None,
                get_composition_string_w: None,
                set_composition_window: None,
                set_candidate_window: None,
            };
        }
        ImeApi {
            get_context: load_proc(h, b"ImmGetContext\0"),
            release_context: load_proc(h, b"ImmReleaseContext\0"),
            get_composition_string_w: load_proc(h, b"ImmGetCompositionStringW\0"),
            set_composition_window: load_proc(h, b"ImmSetCompositionWindow\0"),
            set_candidate_window: load_proc(h, b"ImmSetCandidateWindow\0"),
        }
    })
}

const WM_IME_STARTCOMPOSITION: u32 = 0x010D;
const WM_IME_ENDCOMPOSITION: u32 = 0x010E;
const WM_IME_COMPOSITION: u32 = 0x010F;
const GCS_COMPSTR: u32 = 0x0008;
const GCS_CURSORPOS: u32 = 0x0080;
const GCS_RESULTSTR: u32 = 0x0800;
const CFS_POINT: u32 = 0x0002;
const CFS_CANDIDATEPOS: u32 = 0x0040;

const WM_KEYDOWN: u32 = 0x0100;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_KEYUP: u32 = 0x0101;
const WM_CHAR: u32 = 0x0102;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KILLFOCUS: u32 = 0x0008;
const WM_SIZE: u32 = 0x0005;
const WM_CLOSE: u32 = 0x0010;
const WM_DESTROY: u32 = 0x0002;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_MOUSEWHEEL: u32 = 0x020A;
const WM_DPICHANGED: u32 = 0x02E0;

const VK_SHIFT: u32 = 0x10;
const VK_CONTROL: u32 = 0x11;

// ---- app interface ----

/// One mullion application: called once per frame with the context.
pub trait App {
    fn ui(&mut self, ctx: &mut Ctx);
}

/// Headless verification run.
#[derive(Clone, Debug)]
pub struct SmokeConfig {
    /// Total frames to render.
    pub frames: u32,
    /// After this frame index, resize the window (to exercise realloc).
    pub resize_at: u32,
    /// New logical client size after `resize_at`.
    pub resize_to: (u32, u32),
    /// Where to write the final framebuffer as PNG.
    pub png_path: Option<std::path::PathBuf>,
}

/// Window creation parameters. `size` is the logical client size.
pub struct WindowConfig {
    pub title: String,
    pub size: (u32, u32),
    pub visible: bool,
    pub smoke: Option<SmokeConfig>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "mullion".into(),
            size: (1000, 640),
            visible: true,
            smoke: None,
        }
    }
}

/// Result summary of a run loop.
#[derive(Clone, Debug, Default)]
pub struct RunStats {
    pub frames: u64,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub avg_frame_ms: f64,
    pub opaque_pixels: usize,
    pub fb_hash: u64,
}

// ---- pending events ----

#[derive(Default)]
struct Pending {
    mouse: Option<(f32, f32)>, // physical client px
    primary_down: bool,
    primary_pressed: bool,
    primary_released: bool,
    secondary_down: bool,
    wheel: f32, // accumulated notches
    keys: Vec<KeyEvent>,
    text: String,
    preedit: Option<Preedit>,
    focus_lost: bool,
    resize: Option<(u32, u32)>,
    dpi: Option<f32>,
}

struct Runner {
    hdc: HDC,
    ctx: Ctx,
    fb: Framebuffer,
    gl: Option<WglPresent>,
    app: Box<dyn App>,
    pending: Pending,
    scale: f32,
    logical_size: (u32, u32),
    running: bool,
    frames: u64,
    frame_ms_sum: f64,
    paste_request: Option<mullion::Id>,
    smoke: Option<SmokeConfig>,
    stats: RunStats,
    /// Epoch for [`FrameInput::time_ms`]. The core never reads a clock.
    epoch: Instant,
}

fn get_client_size(hwnd: HWND) -> (u32, u32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe { GetClientRect(hwnd, &mut rc) };
    (
        ((rc.right - rc.left) as u32).max(1),
        ((rc.bottom - rc.top) as u32).max(1),
    )
}

unsafe fn runner_of(hwnd: HWND) -> Option<&'static mut Runner> {
    let p = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd, -21, /* GWLP_USERDATA */
        )
    };
    if p == 0 {
        return None;
    }
    Some(unsafe { &mut *(p as *mut Runner) })
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        let Some(r) = runner_of(hwnd) else {
            return DefWindowProcW(hwnd, msg, wp, lp);
        };
        match msg {
            WM_MOUSEMOVE => {
                r.pending.mouse = Some((lo_short(lp) as f32, hi_short(lp) as f32));
                0
            }
            WM_LBUTTONDOWN => {
                r.pending.mouse = Some((lo_short(lp) as f32, hi_short(lp) as f32));
                r.pending.primary_down = true;
                r.pending.primary_pressed = true;
                SetCapture(hwnd);
                0
            }
            WM_LBUTTONUP => {
                r.pending.mouse = Some((lo_short(lp) as f32, hi_short(lp) as f32));
                r.pending.primary_down = false;
                r.pending.primary_released = true;
                ReleaseCapture();
                0
            }
            WM_RBUTTONDOWN => {
                r.pending.secondary_down = true;
                0
            }
            WM_RBUTTONUP => {
                r.pending.secondary_down = false;
                0
            }
            WM_MOUSEWHEEL => {
                let delta = hi_short(wp as isize) as f32 / 120.0;
                r.pending.wheel += delta;
                // lParam carries screen coords; convert for tracking.
                let mut pt = POINT {
                    x: lo_short(lp),
                    y: hi_short(lp),
                };
                ScreenToClient(hwnd, &mut pt);
                r.pending.mouse = Some((pt.x as f32, pt.y as f32));
                0
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = wp as u32;
                let repeat = (lp >> 30) & 1 == 1;
                if let Some(key) = vk_to_key(vk) {
                    let mods = current_modifiers();
                    r.pending.keys.push(KeyEvent {
                        key,
                        mods,
                        pressed: true,
                        repeat,
                    });
                }
                0
            }
            WM_KEYUP => {
                let vk = wp as u32;
                if let Some(key) = vk_to_key(vk) {
                    let mods = current_modifiers();
                    r.pending.keys.push(KeyEvent {
                        key,
                        mods,
                        pressed: false,
                        repeat: false,
                    });
                }
                0
            }
            WM_CHAR => {
                let c = char::from_u32(wp as u32);
                if let Some(c) = c {
                    if wp >= 0x20 && wp != 0x7f {
                        r.pending.text.push(c);
                    }
                }
                0
            }
            WM_IME_STARTCOMPOSITION => 0,
            WM_IME_ENDCOMPOSITION => {
                r.pending.preedit = None;
                0
            }
            WM_IME_COMPOSITION => {
                handle_ime(r, hwnd, lp);
                0
            }
            WM_SIZE => {
                let w = (lp & 0xFFFF) as u32;
                let h = ((lp >> 16) & 0xFFFF) as u32;
                if w > 0 && h > 0 {
                    r.pending.resize = Some((w, h));
                }
                0
            }
            WM_DPICHANGED => {
                let dpi = (wp & 0xFFFF) as f32;
                r.pending.dpi = Some(dpi / 96.0);
                // Suggested window rect from the OS.
                let suggested = lp as *const RECT;
                if !suggested.is_null() {
                    let rc = *suggested;
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        rc.left,
                        rc.top,
                        rc.right - rc.left,
                        rc.bottom - rc.top,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    );
                }
                0
            }
            WM_KILLFOCUS => {
                r.pending.focus_lost = true;
                r.pending.preedit = None;
                0
            }
            WM_ERASEBKGND => 1, // presentation owns the whole client area
            WM_CLOSE => {
                r.running = false;
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

fn current_modifiers() -> Modifiers {
    unsafe {
        let shift = GetKeyState(VK_SHIFT as i32) as u16 & 0x8000 != 0;
        let ctrl = GetKeyState(VK_CONTROL as i32) as u16 & 0x8000 != 0;
        Modifiers {
            shift,
            ctrl,
            alt: false,
        }
    }
}

fn handle_ime(r: &mut Runner, hwnd: HWND, lp: LPARAM) {
    let api = ime_api();
    let (Some(get_ctx), Some(release_ctx), Some(get_str)) = (
        api.get_context,
        api.release_context,
        api.get_composition_string_w,
    ) else {
        return;
    };
    unsafe {
        let himc = get_ctx(hwnd);
        if himc.is_null() {
            return;
        }
        let read_utf16 = |index: u32| -> Vec<u16> {
            let len = get_str(himc, index, std::ptr::null_mut(), 0);
            if len <= 0 {
                return Vec::new();
            }
            let units = len as usize / 2;
            let mut buf = vec![0u16; units];
            get_str(
                himc,
                index,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                len as u32,
            );
            buf
        };
        if lp as u32 & GCS_RESULTSTR != 0 {
            let s: String = String::from_utf16_lossy(&read_utf16(GCS_RESULTSTR));
            r.pending.text.push_str(&s);
            r.pending.preedit = None;
        }
        if lp as u32 & GCS_COMPSTR != 0 {
            let units = read_utf16(GCS_COMPSTR);
            let pos = get_str(himc, GCS_CURSORPOS, std::ptr::null_mut(), 0);
            let caret = if pos >= 0 { pos as usize } else { 0 };
            let text = String::from_utf16_lossy(&units);
            // Cursor position is in UTF-16 units; clamp to bytes.
            let caret_byte = text
                .char_indices()
                .nth(caret.min(text.chars().count()))
                .map(|(b, _)| b)
                .unwrap_or(text.len());
            r.pending.preedit = Some(Preedit { text, caret_byte });
        }
        release_ctx(hwnd, himc);
    }
}

fn position_ime(hwnd: HWND, caret_logical: Vec2, scale: f32) {
    let api = ime_api();
    let (Some(get_ctx), Some(release_ctx), Some(set_comp), Some(set_cand)) = (
        api.get_context,
        api.release_context,
        api.set_composition_window,
        api.set_candidate_window,
    ) else {
        return;
    };
    unsafe {
        let himc = get_ctx(hwnd);
        if himc.is_null() {
            return;
        }
        let pt = POINT {
            x: (caret_logical.x * scale).round() as i32,
            y: (caret_logical.y * scale).round() as i32,
        };
        let cf = COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: pt,
            rcArea: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
        };
        set_comp(himc, &cf);
        let cand = CANDIDATEFORM {
            dwIndex: 0,
            dwStyle: CFS_CANDIDATEPOS,
            ptCurrentPos: pt,
            rcArea: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
        };
        set_cand(himc, &cand);
        release_ctx(hwnd, himc);
    }
}

/// Run an app until the window closes. Returns run statistics (also used by
/// smoke mode). Returns `None` when the window or GL context could not be
/// created.
pub fn run(app: Box<dyn App>, config: WindowConfig) -> Option<RunStats> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name: Vec<u16> = "mullion_window\0".encode_utf16().collect();
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            return None;
        }

        let style = WS_OVERLAPPEDWINDOW | if config.visible { WS_VISIBLE } else { 0 };
        // Outer rect for the requested logical client size at scale 1.0;
        // the DPI handler resizes right after creation.
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: config.size.0 as i32,
            bottom: config.size.1 as i32,
        };
        AdjustWindowRect(&mut rc, style, 0);
        let title: Vec<u16> = config.title.encode_utf16().chain([0]).collect();
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rc.right - rc.left,
            rc.bottom - rc.top,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return None;
        }

        let hdc = GetDC(hwnd);
        let pfd = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW
                | PFD_SUPPORT_OPENGL
                | PFD_DOUBLEBUFFER
                | PFD_STEREO_DONTCARE,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 0,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 0,
            cStencilBits: 0,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE as u8,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };
        let pf = ChoosePixelFormat(hdc, &pfd);
        if pf == 0 || SetPixelFormat(hdc, pf, &pfd) == 0 {
            ReleaseDC(hwnd, hdc);
            DestroyWindow(hwnd);
            return None;
        }
        let gl = WglPresent::new(hdc);

        let dpi = GetDpiForWindow(hwnd);
        let scale = if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 };

        let fonts = GdiFontSet::new()?;
        let mut ctx = Ctx::new(std::sync::Arc::new(fonts));
        ctx.style.scale = scale;

        // Size the client area to the logical size at the real DPI.
        let (cw, ch) = (config.size.0, config.size.1);
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: (cw as f32 * scale) as i32,
            bottom: (ch as f32 * scale) as i32,
        };
        AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, 0);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            -1,
            -1, // ignored (NOMOVE)
            rc.right - rc.left,
            rc.bottom - rc.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let (w, h) = get_client_size(hwnd);
        let mut runner = Box::new(Runner {
            hdc,
            ctx,
            fb: Framebuffer::new(w, h),
            gl,
            app,
            pending: Pending::default(),
            scale,
            logical_size: config.size,
            running: true,
            frames: 0,
            frame_ms_sum: 0.0,
            paste_request: None,
            smoke: config.smoke.clone(),
            stats: RunStats {
                width: w,
                height: h,
                scale,
                ..Default::default()
            },
            epoch: Instant::now(),
        });
        let runner_ptr: *mut Runner = &mut *runner;
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            -21, // GWLP_USERDATA
            runner_ptr as isize,
        );

        if config.visible {
            ShowWindow(hwnd, SW_SHOW);
        }

        let mut msg: MSG = std::mem::zeroed();
        while runner.running {
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if !runner.running {
                break;
            }

            // Apply queued state changes.
            if let Some(dpi_scale) = runner.pending.dpi.take() {
                runner.scale = dpi_scale;
                runner.ctx.style.scale = dpi_scale;
            }
            if let Some((w, h)) = runner.pending.resize.take() {
                runner.fb.resize(w, h);
                runner.stats.width = w;
                runner.stats.height = h;
                runner.logical_size = (
                    (w as f32 / runner.scale) as u32,
                    (h as f32 / runner.scale) as u32,
                );
            }

            // Build the frame input (logical px).
            let scale = runner.scale;
            let mouse = runner
                .pending
                .mouse
                .map(|(x, y)| Vec2::new(x / scale, y / scale));
            let input = FrameInput {
                primary_down: runner.pending.primary_down,
                primary_pressed: runner.pending.primary_pressed,
                primary_released: runner.pending.primary_released,
                secondary_down: runner.pending.secondary_down,
                mouse_pos: mouse,
                wheel: runner.pending.wheel * 64.0,
                keys: std::mem::take(&mut runner.pending.keys),
                text: std::mem::take(&mut runner.pending.text),
                preedit: runner.pending.preedit.take(),
                paste: runner
                    .paste_request
                    .take()
                    .and_then(|_| clipboard::get_text(hwnd)),
                time_ms: runner.epoch.elapsed().as_secs_f64() * 1000.0,
            };
            let t0 = std::time::Instant::now();

            runner.ctx.begin(input);
            if runner.pending.focus_lost {
                runner.ctx.memory.focus = None;
                runner.pending.focus_lost = false;
            }
            runner.app.ui(&mut runner.ctx);
            let out = runner.ctx.end();

            if let Some(text) = out.copy_text {
                clipboard::set_text(hwnd, &text);
            }
            runner.paste_request = out.paste_id;

            render(&runner.ctx.display, &runner.ctx.shaper, &mut runner.fb);
            if let Some(gl) = runner.gl.as_mut() {
                gl.present(runner.hdc, &runner.fb);
            } else {
                ValidateRect(hwnd, std::ptr::null());
            }
            if let Some(caret) = out.ime_caret {
                position_ime(hwnd, caret, runner.scale);
            }

            // Consume one-frame edge events.
            runner.pending.primary_pressed = false;
            runner.pending.primary_released = false;
            runner.pending.wheel = 0.0;

            let frame_ms = t0.elapsed().as_secs_f64() * 1000.0;
            runner.frame_ms_sum += frame_ms;
            runner.frames += 1;

            if let Some(smoke) = runner.smoke.clone() {
                if runner.frames as u32 == smoke.resize_at {
                    let (lw, lh) = smoke.resize_to;
                    let mut rc = RECT {
                        left: 0,
                        top: 0,
                        right: (lw as f32 * runner.scale) as i32,
                        bottom: (lh as f32 * runner.scale) as i32,
                    };
                    AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, 0);
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        -1,
                        -1,
                        rc.right - rc.left,
                        rc.bottom - rc.top,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                if runner.frames as u32 >= smoke.frames {
                    if let Some(path) = smoke.png_path {
                        let _ = png::write_png(&path, runner.fb.w, runner.fb.h, &runner.fb.px);
                    }
                    runner.running = false;
                }
            }
        }

        runner.stats.frames = runner.frames;
        runner.stats.scale = runner.scale;
        runner.stats.avg_frame_ms = if runner.frames > 0 {
            runner.frame_ms_sum / runner.frames as f64
        } else {
            0.0
        };
        runner.stats.opaque_pixels = runner.fb.opaque_pixels();
        runner.stats.fb_hash = runner.fb.hash();

        // Detach before drop so late messages don't touch freed memory.
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, -21, 0);
        DestroyWindow(hwnd);
        // Release the GL context (and its texture) while the DC is still
        // valid, then the DC, then the window class.
        runner.gl = None;
        ReleaseDC(hwnd, runner.hdc);
        windows_sys::Win32::UI::WindowsAndMessaging::UnregisterClassW(
            class_name.as_ptr(),
            hinstance,
        );
        Some(runner.stats)
    }
}
