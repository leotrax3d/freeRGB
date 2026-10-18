//! CORSAIR K70 CORE RGB (`1b1c:1bfd`) layout.
//!
//! Grid positions describe the physical key arrangement on a 21x6 grid. Wire
//! indices are Corsair's per-key LED indices; they equal the standard USB HID
//! keyboard usage IDs except for the modifier keys and Fn, which Corsair maps
//! to 105..=111 and 122. Source: LED index table for "K70 Core" in the
//! SignalRGB Bragi plugin (factual interface data), cross-checked against the
//! HID Usage Tables for all non-modifier keys. Status: probable until the
//! per-key test in `docs/TESTING.md` has been run.

use freergb_core::{Led, LedLayout};

/// `(name, x, y, wire_index)`
pub const KEYS: &[(&str, u16, u16, u8)] = &[
    // Row 0
    ("Esc", 0, 0, 41),
    ("F1", 2, 0, 58),
    ("F2", 3, 0, 59),
    ("F3", 4, 0, 60),
    ("F4", 5, 0, 61),
    ("F5", 6, 0, 62),
    ("F6", 7, 0, 63),
    ("F7", 8, 0, 64),
    ("F8", 9, 0, 65),
    ("F9", 10, 0, 66),
    ("F10", 11, 0, 67),
    ("F11", 12, 0, 68),
    ("F12", 13, 0, 69),
    ("Print Screen", 14, 0, 70),
    ("Scroll Lock", 15, 0, 71),
    ("Pause", 16, 0, 72),
    // Row 1
    ("`", 0, 1, 53),
    ("1", 1, 1, 30),
    ("2", 2, 1, 31),
    ("3", 3, 1, 32),
    ("4", 4, 1, 33),
    ("5", 5, 1, 34),
    ("6", 6, 1, 35),
    ("7", 7, 1, 36),
    ("8", 8, 1, 37),
    ("9", 9, 1, 38),
    ("0", 10, 1, 39),
    ("-", 11, 1, 45),
    ("=", 12, 1, 46),
    ("Backspace", 13, 1, 42),
    ("Insert", 14, 1, 73),
    ("Home", 15, 1, 74),
    ("Page Up", 16, 1, 75),
    ("Num Lock", 17, 1, 83),
    ("Num /", 18, 1, 84),
    ("Num *", 19, 1, 85),
    ("Num -", 20, 1, 86),
    // Row 2
    ("Tab", 0, 2, 43),
    ("Q", 1, 2, 20),
    ("W", 2, 2, 26),
    ("E", 3, 2, 8),
    ("R", 4, 2, 21),
    ("T", 5, 2, 23),
    ("Y", 6, 2, 28),
    ("U", 7, 2, 24),
    ("I", 8, 2, 12),
    ("O", 9, 2, 18),
    ("P", 10, 2, 19),
    ("[", 11, 2, 47),
    ("]", 12, 2, 48),
    ("\\", 13, 2, 49),
    ("Delete", 14, 2, 76),
    ("End", 15, 2, 77),
    ("Page Down", 16, 2, 78),
    ("Num 7", 17, 2, 95),
    ("Num 8", 18, 2, 96),
    ("Num 9", 19, 2, 97),
    ("Num +", 20, 2, 87),
    // Row 3
    ("Caps Lock", 0, 3, 57),
    ("A", 1, 3, 4),
    ("S", 2, 3, 22),
    ("D", 3, 3, 7),
    ("F", 4, 3, 9),
    ("G", 5, 3, 10),
    ("H", 6, 3, 11),
    ("J", 7, 3, 13),
    ("K", 8, 3, 14),
    ("L", 9, 3, 15),
    (";", 10, 3, 51),
    ("'", 11, 3, 52),
    ("ISO #", 12, 3, 50),
    ("Enter", 13, 3, 40),
    ("Num 4", 17, 3, 92),
    ("Num 5", 18, 3, 93),
    ("Num 6", 19, 3, 94),
    // Row 4
    ("Left Shift", 0, 4, 106),
    ("ISO <", 1, 4, 100),
    ("Z", 2, 4, 29),
    ("X", 3, 4, 27),
    ("C", 4, 4, 6),
    ("V", 5, 4, 25),
    ("B", 6, 4, 5),
    ("N", 7, 4, 17),
    ("M", 8, 4, 16),
    (",", 9, 4, 54),
    (".", 10, 4, 55),
    ("/", 11, 4, 56),
    ("ABNT2 /", 12, 4, 115),
    ("Right Shift", 13, 4, 110),
    ("Up", 15, 4, 82),
    ("Num 1", 17, 4, 89),
    ("Num 2", 18, 4, 90),
    ("Num 3", 19, 4, 91),
    ("Num Enter", 20, 4, 88),
    // Row 5
    ("Left Ctrl", 0, 5, 105),
    ("Left Win", 1, 5, 108),
    ("Left Alt", 2, 5, 107),
    ("Space", 6, 5, 44),
    ("Right Alt", 10, 5, 111),
    ("Fn", 11, 5, 122),
    ("Menu", 12, 5, 101),
    ("Right Ctrl", 13, 5, 109),
    ("Left", 14, 5, 80),
    ("Down", 15, 5, 81),
    ("Right", 16, 5, 79),
    ("Num 0", 17, 5, 98),
    ("Num .", 19, 5, 99),
];

pub const WIDTH: u16 = 21;
pub const HEIGHT: u16 = 6;

pub fn layout() -> LedLayout {
    let leds = KEYS
        .iter()
        .map(|&(name, x, y, _)| Led {
            name: name.to_owned(),
            pos: (x, y),
        })
        .collect();
    LedLayout::new(WIDTH, HEIGHT, leds).expect("static K70 CORE layout is valid")
}

pub fn wire_map() -> Vec<u16> {
    KEYS.iter().map(|&(_, _, _, w)| u16::from(w)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn layout_is_valid_and_unique() {
        let l = layout();
        assert_eq!(l.len(), KEYS.len());
        let wires: HashSet<_> = KEYS.iter().map(|k| k.3).collect();
        assert_eq!(wires.len(), KEYS.len(), "wire indices must be unique");
        let cells: HashSet<_> = KEYS.iter().map(|k| (k.1, k.2)).collect();
        assert_eq!(cells.len(), KEYS.len(), "grid cells must be unique");
    }

    #[test]
    fn non_modifier_indices_match_hid_usage_ids() {
        // Spot checks against the USB HID Usage Tables (Keyboard page 0x07).
        let usage = |n: &str| KEYS.iter().find(|k| k.0 == n).unwrap().3;
        assert_eq!(usage("A"), 0x04);
        assert_eq!(usage("Esc"), 0x29);
        assert_eq!(usage("Space"), 0x2C);
        assert_eq!(usage("F12"), 0x45);
        assert_eq!(usage("Num 0"), 0x62);
        assert_eq!(usage("ISO <"), 0x64);
    }
}
