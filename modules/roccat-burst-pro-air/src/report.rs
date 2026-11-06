// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: RoccatBurstProAirController.cpp SetDirectMode / report
pub const REPORT_LEN: usize = 30;
pub fn direct(colors: &[Rgb; 4]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x06;
    p[1] = 0x01;
    p[2] = 0x4C;
    p[3] = 0x06;
    p[4] = 0x14;
    p[5] = 0x01;
    p[6] = 0xFF;
    p[7] = colors[0].r;
    p[8] = colors[0].g;
    p[9] = colors[0].b;
    p[10] = 0x02;
    p[11] = 0xFF;
    p[12] = colors[1].r;
    p[13] = colors[1].g;
    p[14] = colors[1].b;
    p[15] = 0x03;
    p[16] = 0xFF;
    p[17] = colors[2].r;
    p[18] = colors[2].g;
    p[19] = colors[2].b;
    p[20] = 0x04;
    p[21] = 0xFF;
    p[22] = colors[3].r;
    p[23] = colors[3].g;
    p[24] = colors[3].b;
    p
}
