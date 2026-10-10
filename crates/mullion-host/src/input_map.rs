//! Pure virtual-key -> egui key mapping, unit tested without a window.

/// Map a Win32 virtual-key code to an [`egui::Key`].
pub fn vk_to_key(vk: u32) -> Option<egui::Key> {
    use egui::Key;
    match vk {
        0x25 => Some(Key::ArrowLeft),
        0x26 => Some(Key::ArrowUp),
        0x27 => Some(Key::ArrowRight),
        0x28 => Some(Key::ArrowDown),
        0x24 => Some(Key::Home),
        0x23 => Some(Key::End),
        0x22 => Some(Key::PageUp),
        0x21 => Some(Key::PageDown),
        0x08 => Some(Key::Backspace),
        0x2E => Some(Key::Delete),
        0x0D => Some(Key::Enter),
        0x1B => Some(Key::Escape),
        0x09 => Some(Key::Tab),
        0x30 => Some(Key::Num0),
        0x31 => Some(Key::Num1),
        0x32 => Some(Key::Num2),
        0x33 => Some(Key::Num3),
        0x34 => Some(Key::Num4),
        0x35 => Some(Key::Num5),
        0x36 => Some(Key::Num6),
        0x37 => Some(Key::Num7),
        0x38 => Some(Key::Num8),
        0x39 => Some(Key::Num9),
        0x41..=0x5A => match vk {
            0x41 => Some(Key::A),
            0x42 => Some(Key::B),
            0x43 => Some(Key::C),
            0x44 => Some(Key::D),
            0x45 => Some(Key::E),
            0x46 => Some(Key::F),
            0x47 => Some(Key::G),
            0x48 => Some(Key::H),
            0x49 => Some(Key::I),
            0x4A => Some(Key::J),
            0x4B => Some(Key::K),
            0x4C => Some(Key::L),
            0x4D => Some(Key::M),
            0x4E => Some(Key::N),
            0x4F => Some(Key::O),
            0x50 => Some(Key::P),
            0x51 => Some(Key::Q),
            0x52 => Some(Key::R),
            0x53 => Some(Key::S),
            0x54 => Some(Key::T),
            0x55 => Some(Key::U),
            0x56 => Some(Key::V),
            0x57 => Some(Key::W),
            0x58 => Some(Key::X),
            0x59 => Some(Key::Y),
            _ => Some(Key::Z),
        },
        _ => None,
    }
}

/// Translate a Win32 cursor icon request to an IDC_* resource id.
pub fn egui_cursor_to_idc(icon: egui::CursorIcon) -> *const u16 {
    // IDC_* constants are MAKEINTRESOURCE values; windows-sys exports them
    // as *const u16 already, but this module stays pure so we reinterpret
    // the well-known ordinal values.
    let idc: u16 = match icon {
        egui::CursorIcon::PointingHand => 32649, // IDC_HAND
        egui::CursorIcon::Text | egui::CursorIcon::VerticalText => 32513, // IDC_IBEAM
        egui::CursorIcon::Crosshair => 32515,    // IDC_CROSS
        egui::CursorIcon::ResizeVertical => 32645, // IDC_SIZENS
        egui::CursorIcon::ResizeHorizontal => 32644, // IDC_SIZEWE
        egui::CursorIcon::ResizeNeSw => 32643,   // IDC_SIZENESW
        egui::CursorIcon::ResizeNwSe => 32642,   // IDC_SIZENWSE
        egui::CursorIcon::Move | egui::CursorIcon::AllScroll => 32646, // IDC_SIZEALL
        egui::CursorIcon::NotAllowed | egui::CursorIcon::NoDrop => 32648, // IDC_NO
        egui::CursorIcon::Wait => 32514,         // IDC_WAIT
        egui::CursorIcon::Progress => 32650,     // IDC_APPSTARTING
        egui::CursorIcon::Grab | egui::CursorIcon::Grabbing => 32646, // IDC_SIZEALL
        _ => 32512,                              // IDC_ARROW
    };
    idc as usize as *const u16
}

/// Signed 16-bit extraction from an LPARAM (mouse coordinates).
pub fn lo_short(l: isize) -> i32 {
    (l as u16 as i16) as i32
}

pub fn hi_short(l: isize) -> i32 {
    ((l >> 16) as u16 as i16) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_keys() {
        assert_eq!(vk_to_key(0x25), Some(egui::Key::ArrowLeft));
        assert_eq!(vk_to_key(0x0D), Some(egui::Key::Enter));
        assert_eq!(vk_to_key(0x41), Some(egui::Key::A));
        assert_eq!(vk_to_key(0x58), Some(egui::Key::X));
        assert_eq!(vk_to_key('Q' as u32), Some(egui::Key::Q));
        assert_eq!(vk_to_key(0x70 /* F1 */), None);
    }

    #[test]
    fn cursor_icons_map_to_idc() {
        assert_eq!(
            egui_cursor_to_idc(egui::CursorIcon::Default),
            32512usize as *const u16
        );
        assert_eq!(
            egui_cursor_to_idc(egui::CursorIcon::PointingHand),
            32649usize as *const u16
        );
        assert_eq!(
            egui_cursor_to_idc(egui::CursorIcon::Text),
            32513usize as *const u16
        );
    }

    #[test]
    fn extracts_signed_coords() {
        // (-5, 300)
        let lp = (((-5i32) as u16 as u32) | (300u32 << 16)) as usize as isize;
        assert_eq!(lo_short(lp), -5);
        assert_eq!(hi_short(lp), 300);
    }
}
