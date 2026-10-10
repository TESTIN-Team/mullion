//! Win32 window + run loop for egui apps: message pump translated to
//! `egui::RawInput`, per-monitor DPI, IME composition, clipboard glue,
//! vsync presentation through the GL 1.1 painter, and a headless `--smoke`
//! mode that renders fixed frames and captures the backbuffer as PNG.

use crate::clipboard;
use crate::input_map::{egui_cursor_to_idc, hi_short, lo_short, vk_to_key};
use crate::paint::EguiPainter;
use crate::png;
use crate::wgl::GlContext;
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
    SWP_NOZORDER, SetCursor, SetWindowPos, ShowWindow, TranslateMessage, WNDCLASSW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
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
const GCS_RESULTSTR: u32 = 0x0800;
const CFS_POINT: u32 = 0x0002;
const CFS_CANDIDATEPOS: u32 = 0x0040;

const WM_KEYDOWN: u32 = 0x0100;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_KEYUP: u32 = 0x0101;
const WM_CHAR: u32 = 0x0102;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KILLFOCUS: u32 = 0x0008;
const WM_SETFOCUS: u32 = 0x0007;
const WM_SETCURSOR: u32 = 0x0020;
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
const VK_V: u32 = 0x56;

// ---- app interface ----

/// One egui application: called once per frame with the context.
pub type EguiApp = Box<dyn FnMut(&egui::Context)>;

/// Headless verification run.
#[derive(Clone, Debug)]
pub struct SmokeConfig {
    /// Total frames to render.
    pub frames: u32,
    /// After this frame index, resize the window (to exercise realloc).
    pub resize_at: u32,
    /// New logical client size after `resize_at`.
    pub resize_to: (u32, u32),
    /// Where to write the final backbuffer capture as PNG.
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
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    resize: Option<(u32, u32)>,
    dpi: Option<f32>,
}

fn current_modifiers() -> egui::Modifiers {
    unsafe {
        let shift = GetKeyState(VK_SHIFT as i32) as u16 & 0x8000 != 0;
        let ctrl = GetKeyState(VK_CONTROL as i32) as u16 & 0x8000 != 0;
        egui::Modifiers {
            alt: false,
            ctrl,
            shift,
            mac_cmd: false,
            command: ctrl,
        }
    }
}

struct Runner {
    hdc: HDC,
    ctx: egui::Context,
    painter: EguiPainter,
    gl: Option<GlContext>,
    app: EguiApp,
    pending: Pending,
    scale: f32,
    logical_size: (u32, u32),
    running: bool,
    frames: u64,
    frame_ms_sum: f64,
    cursor_icon: egui::CursorIcon,
    smoke: Option<SmokeConfig>,
    stats: RunStats,
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
        let scale = r.scale;
        let m = &mut r.pending;
        let pos_of = |lp: LPARAM| {
            egui::Pos2::new((lo_short(lp) as f32) / scale, (hi_short(lp) as f32) / scale)
        };
        match msg {
            WM_MOUSEMOVE => {
                let pos = pos_of(lp);
                m.events.push(egui::Event::PointerMoved(pos));
                0
            }
            WM_LBUTTONDOWN => {
                SetCapture(hwnd);
                let pos = pos_of(lp);
                m.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: m.modifiers,
                });
                0
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
                let pos = pos_of(lp);
                m.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: m.modifiers,
                });
                0
            }
            WM_RBUTTONDOWN | WM_RBUTTONUP => {
                let pos = pos_of(lp);
                m.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: msg == WM_RBUTTONDOWN,
                    modifiers: m.modifiers,
                });
                0
            }
            WM_MOUSEWHEEL => {
                let delta = hi_short(wp as isize) as f32 / 120.0;
                let mut pt = POINT {
                    x: lo_short(lp),
                    y: hi_short(lp),
                };
                ScreenToClient(hwnd, &mut pt);
                let pos = egui::Pos2::new(pt.x as f32 / scale, pt.y as f32 / scale);
                m.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::Vec2::new(0.0, -delta * 20.0),
                    modifiers: m.modifiers,
                });
                m.events.push(egui::Event::PointerMoved(pos));
                0
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = wp as u32;
                let repeat = (lp >> 30) & 1 == 1;
                m.modifiers = current_modifiers();
                if let Some(key) = vk_to_key(vk) {
                    m.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat,
                        modifiers: m.modifiers,
                    });
                    if vk == VK_V && m.modifiers.ctrl {
                        let text = crate::clipboard::get_text(hwnd).unwrap_or_default();
                        m.events.push(egui::Event::Paste(text));
                    }
                }
                0
            }
            WM_KEYUP => {
                let vk = wp as u32;
                m.modifiers = current_modifiers();
                if let Some(key) = vk_to_key(vk) {
                    m.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: m.modifiers,
                    });
                }
                0
            }
            WM_CHAR => {
                if let Some(c) = char::from_u32(wp as u32) {
                    if wp >= 0x20 && wp != 0x7f {
                        m.events.push(egui::Event::Text(c.to_string()));
                    }
                }
                0
            }
            WM_IME_STARTCOMPOSITION => {
                m.events.push(egui::Event::Ime(egui::ImeEvent::Enabled));
                0
            }
            WM_IME_ENDCOMPOSITION => {
                m.events.push(egui::Event::Ime(egui::ImeEvent::Disabled));
                0
            }
            WM_IME_COMPOSITION => {
                handle_ime(m, hwnd, lp);
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
            WM_SETFOCUS => {
                m.events.push(egui::Event::WindowFocused(true));
                0
            }
            WM_KILLFOCUS => {
                m.events.push(egui::Event::WindowFocused(false));
                m.modifiers = egui::Modifiers::default();
                0
            }
            WM_SETCURSOR => {
                // HTCLIENT == 1: the message concerns the client area.
                if (lp as u32 & 0xFFFF) == 1 {
                    let idc = egui_cursor_to_idc(r.cursor_icon);
                    let cur = LoadCursorW(std::ptr::null_mut(), idc);
                    if !cur.is_null() {
                        SetCursor(cur);
                        return 1;
                    }
                }
                DefWindowProcW(hwnd, msg, wp, lp)
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

fn handle_ime(m: &mut Pending, hwnd: HWND, lp: LPARAM) {
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
            let s = String::from_utf16_lossy(&read_utf16(GCS_RESULTSTR));
            m.events.push(egui::Event::Ime(egui::ImeEvent::Commit(s)));
        }
        if lp as u32 & GCS_COMPSTR != 0 {
            let text = String::from_utf16_lossy(&read_utf16(GCS_COMPSTR));
            m.events
                .push(egui::Event::Ime(egui::ImeEvent::Preedit(text)));
        }
        release_ctx(hwnd, himc);
    }
}

fn position_ime(hwnd: HWND, caret_logical: (f32, f32), scale: f32) {
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
            x: (caret_logical.0 * scale).round() as i32,
            y: (caret_logical.1 * scale).round() as i32,
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

/// Add a system CJK font to egui so Chinese UI text renders. Tries the
/// common Windows font files in order; falls back to egui defaults.
fn install_cjk_fonts(ctx: &egui::Context) -> bool {
    let candidates = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\deng.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];
    for path in candidates {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if bytes.is_empty() {
            continue;
        }
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "cjk".into(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .expect("proportional family exists")
            .insert(0, "cjk".into());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .expect("monospace family exists")
            .push("cjk".into());
        ctx.set_fonts(fonts);
        return true;
    }
    false
}

/// Run an egui app until the window closes. Returns run statistics (also
/// used by smoke mode), or `None` when the window or GL context could not
/// be created.
pub fn run_egui(app: EguiApp, config: WindowConfig) -> Option<RunStats> {
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
        let gl = GlContext::new(hdc);

        let dpi = GetDpiForWindow(hwnd);
        let scale = if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 };

        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        install_cjk_fonts(&ctx);

        // Size the client area to the logical size at the real DPI.
        let (cw, ch) = config.size;
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
            painter: EguiPainter::new(),
            gl,
            app,
            pending: Pending::default(),
            scale,
            logical_size: config.size,
            running: true,
            frames: 0,
            frame_ms_sum: 0.0,
            cursor_icon: egui::CursorIcon::Default,
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

            if let Some(dpi_scale) = runner.pending.dpi.take() {
                runner.scale = dpi_scale;
            }
            if let Some((w, h)) = runner.pending.resize.take() {
                runner.stats.width = w;
                runner.stats.height = h;
                runner.logical_size = (
                    (w as f32 / runner.scale) as u32,
                    (h as f32 / runner.scale) as u32,
                );
            }

            let scale = runner.scale;
            let (lw, lh) = runner.logical_size;
            let t0 = Instant::now();
            let capture_now = runner
                .smoke
                .as_ref()
                .is_some_and(|s| runner.frames as u32 + 1 >= s.frames);

            let raw = egui::RawInput {
                events: std::mem::take(&mut runner.pending.events),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(lw as f32, lh as f32),
                )),
                time: Some(runner.epoch.elapsed().as_secs_f64()),
                modifiers: runner.pending.modifiers,
                ..Default::default()
            };
            runner.ctx.set_pixels_per_point(scale);
            let app = &mut runner.app;
            let output = runner.ctx.run(raw, |ctx| app(ctx));

            // Platform output: clipboard, cursor icon, IME caret rectangle.
            for cmd in &output.platform_output.commands {
                if let egui::OutputCommand::CopyText(text) = cmd {
                    clipboard::set_text(hwnd, text);
                }
            }
            runner.cursor_icon = output.platform_output.cursor_icon;
            if let Some(ime) = &output.platform_output.ime {
                let rect = ime.cursor_rect;
                position_ime(hwnd, (rect.left(), rect.top()), scale);
            }

            // Apply texture updates (font atlas arrives here too) BEFORE
            // painting, then tessellate and draw.
            runner.painter.set_textures(&output.textures_delta);
            let primitives = runner.ctx.tessellate(output.shapes, scale);
            let mut capture: Option<Vec<u8>> = capture_now.then(Vec::new);
            if runner.gl.is_some() {
                runner.painter.paint_and_swap(
                    runner.hdc,
                    &primitives,
                    (runner.stats.width as i32, runner.stats.height as i32),
                    capture.as_mut(),
                );
            } else {
                ValidateRect(hwnd, std::ptr::null());
            }

            let frame_ms = t0.elapsed().as_secs_f64() * 1000.0;
            runner.frame_ms_sum += frame_ms;
            runner.frames += 1;

            if let Some(buf) = capture.take() {
                runner.stats.opaque_pixels = buf.chunks_exact(4).filter(|c| c[3] != 0).count();
                runner.stats.fb_hash = fnv1a(&buf);
                if let Some(smoke) = runner.smoke.as_ref() {
                    if let Some(path) = smoke.png_path.clone() {
                        let _ =
                            png::write_png(&path, runner.stats.width, runner.stats.height, &buf);
                    }
                }
            }

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

        // Detach before drop so late messages don't touch freed memory.
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, -21, 0);
        DestroyWindow(hwnd);
        // Release painter texture and the GL context while the DC is valid.
        runner.painter = EguiPainter::new();
        runner.gl = None;
        ReleaseDC(hwnd, runner.hdc);
        windows_sys::Win32::UI::WindowsAndMessaging::UnregisterClassW(
            class_name.as_ptr(),
            hinstance,
        );
        Some(runner.stats)
    }
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}
