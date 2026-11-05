// SPDX-License-Identifier: GPL-2.0-or-later
/// upstream: TecknetController.cpp SendUpdate (Direct mode)
pub const REPORT_LEN: usize = 16;
pub fn direct(c: freergb_core::Rgb) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x02;
    p[1] = 0x04;
    p[2] = 255 - c.r;
    p[3] = 255 - c.g;
    p[4] = 255 - c.b;
    p[5] = 0x03; // high brightness
    p[6] = 0x00; // direct speed (off)
    p
}