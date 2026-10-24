// SPDX-License-Identifier: GPL-2.0-or-later
// Ported from OpenRGB RedragonMouseController.cpp (GPL-2.0-or-later).
pub const REPORT_ID: u8 = 0x02;
pub const REPORT_LEN: usize = 16;
pub const MODE_STATIC: u8 = 0x02;

pub fn write_packet(address: u16, data: &[u8]) -> [u8; REPORT_LEN] {
    assert!(data.len() <= 8);
    let mut p = [0; REPORT_LEN];
    p[0] = REPORT_ID;
    p[1] = 0xF3;
    p[2..4].copy_from_slice(&address.to_le_bytes());
    p[4] = data.len() as u8;
    p[8..8 + data.len()].copy_from_slice(data);
    p
}
pub fn apply() -> [u8; REPORT_LEN] {
    [
        REPORT_ID, 0xF1, 0x02, 0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
}
pub fn profile_zero() -> [u8; REPORT_LEN] {
    write_packet(0x002C, &[0])
}
pub fn color(r: u8, g: u8, b: u8) -> [u8; REPORT_LEN] {
    write_packet(0x0449, &[r, g, b])
}
