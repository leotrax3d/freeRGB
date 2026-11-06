// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: AcerNitroHidKeyboardController.cpp SetLEDs (device_id 0x21, Direct mapped via SetLEDs)
pub const REPORT_LEN: usize = 11;
pub fn direct(led_idx: usize, c: Rgb) -> [u8; REPORT_LEN] {
    [
        0xA4,
        0x21,
        0x02,
        100,
        0x00,
        0x01,
        c.r,
        c.g,
        c.b,
        (1 << led_idx) as u8,
        0x00,
    ]
}
