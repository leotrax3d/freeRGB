// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 8;
pub fn init() -> [u8; REPORT_LEN] {
    let mut b = [0u8; REPORT_LEN];
    b[0] = 0x07;
    b[1] = 0xA0;
    b
}
pub fn color(c: Rgb) -> [u8; REPORT_LEN] {
    let mut b = [0u8; REPORT_LEN];
    b[0] = 0x07;
    b[1] = 0x0B;
    b[2] = 0x01;
    b[3] = c.r;
    b[4] = c.g;
    b[5] = c.b;
    b
}
pub fn mode_direct() -> [u8; REPORT_LEN] {
    let mut b = [0u8; REPORT_LEN];
    b[0] = 0x07;
    b[1] = 0x0A;
    b[2] = 0x02;
    b[3] = 0x01;
    b[4] = 0x01;
    b[5] = 0x01;
    b[6] = 0x01; // speed min 1
    b[7] = 100; // brightness max 100
    b
}
