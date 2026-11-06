// SPDX-License-Identifier: GPL-2.0-or-later
use crate::positions::POSITIONS;
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 1025;
pub fn init_packets() -> [Vec<u8>; 3] {
    let mut a = vec![0u8; REPORT_LEN];
    a[1] = 0x08;
    a[2] = 0x01;
    a[3] = 0x03;
    a[5] = 0x02;
    let mut b = vec![0u8; REPORT_LEN];
    b[1] = 0x08;
    b[2] = 0x02;
    b[3] = 0x6E;
    let mut c = vec![0u8; REPORT_LEN];
    c[1] = 0x08;
    c[2] = 0x0D;
    c[3] = 0x01;
    c[4] = 0x22;
    [a, b, c]
}
pub fn frame(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_LEN];
    p[1] = 0x08;
    p[2] = 0x06;
    p[3] = 0x01;
    p[4] = 0x73;
    p[5] = 0x01;
    p[8] = 0x12;
    for (i, color) in colors.iter().enumerate() {
        if i < POSITIONS.len() {
            let off = 10 + POSITIONS[i] * 3;
            if off + 2 < p.len() {
                p[off] = color.r;
                p[off + 1] = color.g;
                p[off + 2] = color.b;
            }
        }
    }
    p
}
