// SPDX-License-Identifier: GPL-2.0-or-later
pub const REPORT_LEN: usize = 65;
pub const LEDS_PER_PACKET: usize = 20;
pub fn config_request() -> [u8; REPORT_LEN] {
    let mut p = [0; REPORT_LEN];
    p[0] = 0xec;
    p[1] = 0xb0;
    p
}
pub fn direct(
    channel: u8,
    offset: usize,
    colors: &[freergb_core::Rgb],
    apply: bool,
) -> [u8; REPORT_LEN] {
    let mut p = [0; REPORT_LEN];
    p[0] = 0xec;
    p[1] = 0x40;
    p[2] = channel | if apply { 0x80 } else { 0 };
    p[3] = (offset & 0xff) as u8;
    p[4] = colors.len() as u8;
    for (i, c) in colors.iter().enumerate() {
        let q = 5 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
