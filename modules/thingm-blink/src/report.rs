// SPDX-License-Identifier: GPL-2.0-or-later
// Ported from OpenRGB BlinkController.cpp (GPL-2.0-or-later).
pub const REPORT_LEN: usize = 9;
pub fn direct(led: u8, r: u8, g: u8, b: u8) -> [u8; REPORT_LEN] {
    [1, 0x63, r, g, b, 0, 0, led, 0]
}
