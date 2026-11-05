// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct_keyboard(c: Rgb, idx: usize) -> [u8; 9] {
    let mut p = [0u8; 9];
    p[0] = 0x00;
    p[1] = 0x14;
    p[2] = 0x00;
    p[3] = (idx + 1) as u8;
    p[4] = c.r;
    p[5] = c.g;
    p[6] = c.b;
    p
}
pub fn keyboard_mode_direct() -> [u8; 9] {
    [0x00, 0x08, 0x02, 0x01, 0x00, 0x32, 0x08, 0x00, 0x00]
}
pub fn lightbar_init0() -> [u8; 9] {
    [0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
}
pub fn lightbar_init1() -> [u8; 9] {
    [0x00, 0x12, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]
}
pub fn lightbar_mode_direct() -> [u8; 9] {
    [0x00, 0x08, 0x02, 0x33, 0x00, 0x32, 0x08, 0x00, 0x00]
}
pub fn lightbar_write(colors: &[Rgb]) -> [u8; 65] {
    let mut p = [0u8; 65];
    for (i, c) in colors.iter().enumerate() {
        let off = 1 + i * 3;
        if off + 2 < 65 {
            p[off] = c.r;
            p[off + 1] = c.b;
            p[off + 2] = c.g;
        } else if off == 64 {
            p[64] = c.r;
        }
    }
    p
}
