//! Pure virtual-key -> semantic key mapping, unit tested without a window.

use mullion::input::Key;

/// Map a Win32 virtual-key code to a [`Key`].
pub fn vk_to_key(vk: u32) -> Option<Key> {
    match vk {
        0x25 => Some(Key::Left),
        0x26 => Some(Key::Up),
        0x27 => Some(Key::Right),
        0x28 => Some(Key::Down),
        0x24 => Some(Key::Home),
        0x23 => Some(Key::End),
        0x08 => Some(Key::Backspace),
        0x2E => Some(Key::Delete),
        0x0D => Some(Key::Enter),
        0x1B => Some(Key::Escape),
        0x09 => Some(Key::Tab),
        0x41 => Some(Key::A),
        0x43 => Some(Key::C),
        0x56 => Some(Key::V),
        0x58 => Some(Key::X),
        _ => None,
    }
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
        assert_eq!(vk_to_key(0x25), Some(Key::Left));
        assert_eq!(vk_to_key(0x0D), Some(Key::Enter));
        assert_eq!(vk_to_key(0x41), Some(Key::A));
        assert_eq!(vk_to_key('Q' as u32), None);
    }

    #[test]
    fn extracts_signed_coords() {
        // (-5, 300)
        let lp = (((-5i32) as u16 as u32) | (300u32 << 16)) as usize as isize;
        assert_eq!(lo_short(lp), -5);
        assert_eq!(hi_short(lp), 300);
    }
}
