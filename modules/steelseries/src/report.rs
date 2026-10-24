// SPDX-License-Identifier: GPL-2.0-or-later
pub const REPORT_LEN: usize = 64;
pub fn direct(colors: &[freergb_core::Rgb]) -> [u8; REPORT_LEN] {
    let mut p = [0; REPORT_LEN];
    p[0] = 0x00;
    p[1] = 0x02;
    for (i, c) in colors.iter().take(20).enumerate() {
        let q = 2 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
