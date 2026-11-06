// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream constants from `RoccatEloController.h`.
pub const REPORT_SIZE: usize = 16;

/// Upstream: RoccatEloController.cpp SendInit (called from the controller
/// constructor): five 16-byte output writes in order.
pub fn init() -> Vec<Vec<u8>> {
    let mut p1 = vec![0u8; REPORT_SIZE];
    p1[0] = 0xff;
    p1[1] = 0x01;
    let mut p2 = vec![0u8; REPORT_SIZE];
    p2[0] = 0xff;
    p2[1] = 0x02;
    let mut p3 = vec![0u8; REPORT_SIZE];
    p3[0] = 0xff;
    p3[1] = 0x03;
    p3[3] = 0x01;
    let p4 = direct(&Rgb { r: 0, g: 0, b: 0 });
    let mut p5 = vec![0u8; REPORT_SIZE];
    p5[0] = 0xff;
    p5[1] = 0x01;
    vec![p1, p2, p3, p4, p5]
}

/// Upstream: RoccatEloController.cpp SendDirect: 16-byte output report
/// with the single RGB triplet at offset 4.
pub fn direct(c: &Rgb) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_SIZE];
    p[0] = 0xff;
    p[1] = 0x04;
    p[4] = c.r;
    p[5] = c.g;
    p[6] = c.b;
    p
}
