// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: DreamCheekyController.cpp constructor init packets.
pub const REPORT_LEN: usize = 9;
pub fn init_packets() -> [[u8; REPORT_LEN]; 4] {
    [
        [0x00, 0x1F, 0x02, 0x00, 0x5F, 0x00, 0x00, 0x1F, 0x03],
        [0x00, 0x00, 0x02, 0x00, 0x5F, 0x00, 0x00, 0x1F, 0x04],
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F, 0x05],
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
    ]
}
/// upstream: DreamCheekyController.cpp SetColor()
/// Scaling: 0..255 -> 0..64 (>> 2, plus 1 when input == 255).
pub fn color(c: Rgb) -> [u8; REPORT_LEN] {
    let scale = |v: u8| -> u8 { (v >> 2) + if v == 255 { 1 } else { 0 } };
    [
        0x00,
        scale(c.r),
        scale(c.g),
        scale(c.b),
        0x00,
        0x00,
        0x00,
        0x1F,
        0x05,
    ]
}
