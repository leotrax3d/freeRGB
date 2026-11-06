// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(c: Rgb) -> [u8; 8] {
    [0xCD, 0x00, c.r, c.g, c.b, 0x00, 0x00, 0x00]
}
