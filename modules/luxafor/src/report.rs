// SPDX-License-Identifier: GPL-2.0-or-later
// Ported from OpenRGB Controllers/LuxaforController/LuxaforController.cpp.
pub const REPORT_LEN: usize = 9;
pub const MODE_DIRECT: u8 = 1;
pub const LED_FIRST: u8 = 1;

pub fn direct(led: u8, r: u8, g: u8, b: u8) -> [u8; REPORT_LEN] {
    [0, MODE_DIRECT, led, r, g, b, 100, 0, 0]
}
