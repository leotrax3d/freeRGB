// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: CreativeSoundBlasterXG6Controller.cpp SetLedColor
pub const REPORT_LEN: usize = 64;
pub fn packet1() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x5A;
    p[2] = 0x3A;
    p[3] = 0x02;
    p[4] = 0x06;
    p[5] = 0x01;
    p
}
pub fn packet2() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x5A;
    p[2] = 0x3A;
    p[3] = 0x06;
    p[4] = 0x04;
    p[6] = 0x03;
    p[7] = 0x01;
    p[9] = 0x01;
    p
}
pub fn packet3(c: Rgb) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x5A;
    p[2] = 0x3A;
    p[3] = 0x09;
    p[4] = 0x0A;
    p[6] = 0x03;
    p[7] = 0x01;
    p[8] = 0x01;
    p[9] = 0xFF; // fixed brightness
    p[10] = c.b;
    p[11] = c.g;
    p[12] = c.r;
    p
}
