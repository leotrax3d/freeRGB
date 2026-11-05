// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: AMDWraithPrismController.cpp SendPacket (Direct packet)
pub const REPORT_LEN: usize = 65;
pub const LOGO_FAN_IDS: [u8; 2] = [0x00, 0x01];
pub const RING_IDS: [u8; 15] = [
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09,
];
pub fn enable_direct() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x41;
    p[2] = 0x03;
    p
}
pub fn apply() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x51;
    p[2] = 0x28;
    p
}
pub fn packet(ids: &[u8], colors: &[Rgb]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0xC0;
    p[2] = 0x01;
    p[3] = colors.len() as u8;
    for (i, c) in colors.iter().enumerate() {
        let o = 5 + i * 4;
        p[o] = ids[i];
        p[o + 1] = c.r;
        p[o + 2] = c.g;
        p[o + 3] = c.b;
    }
    p
}
