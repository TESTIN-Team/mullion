//! Unicode-text clipboard access (CF_UNICODETEXT).

use windows_sys::Win32::Foundation::{GlobalFree, HANDLE};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};

pub const CF_UNICODETEXT: u32 = 13;

/// Place UTF-16 text on the clipboard. Returns false when the clipboard
/// could not be opened or written.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn set_text(hwnd: windows_sys::Win32::Foundation::HWND, text: &str) -> bool {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return false;
        }
        let ok = (|| {
            if EmptyClipboard() == 0 {
                return false;
            }
            let bytes = units.len() * 2;
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if h.is_null() {
                return false;
            }
            let dst = GlobalLock(h);
            if dst.is_null() {
                GlobalFree(h);
                return false;
            }
            std::ptr::copy_nonoverlapping(units.as_ptr(), dst as *mut u16, units.len());
            GlobalUnlock(h);
            // Ownership transfers to the clipboard on success.
            !SetClipboardData(CF_UNICODETEXT, h as HANDLE).is_null()
        })();
        CloseClipboard();
        ok
    }
}

/// Read UTF-16 text from the clipboard (truncated at the first NUL,
/// capped at 1 MiB).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn get_text(hwnd: windows_sys::Win32::Foundation::HWND) -> Option<String> {
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return None;
        }
        let out = (|| {
            let h = GetClipboardData(CF_UNICODETEXT);
            if h.is_null() {
                return None;
            }
            let src = GlobalLock(h) as *const u16;
            if src.is_null() {
                return None;
            }
            let mut units = Vec::new();
            for i in 0..(512 * 1024) {
                let u = *src.add(i);
                if u == 0 {
                    break;
                }
                units.push(u);
            }
            GlobalUnlock(h);
            Some(String::from_utf16_lossy(&units))
        })();
        CloseClipboard();
        out
    }
}
