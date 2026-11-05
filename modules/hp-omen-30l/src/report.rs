// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: HPOmen30LController.cpp SendZoneUpdate (Direct mode, 58-byte packet)
pub const REPORT_LEN: usize = 58;
pub fn direct(zone_value: u8, c: Rgb) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[2] = 0x12; // version id
    p[3] = 0x04; // direct mode
    p[4] = 0x01;
    p[5] = 0x01;
    let idx = (zone_value as usize).saturating_sub(1);
    p[8 + idx * 4] = 0x64; // max brightness channel
    p[9 + idx * 4] = c.r;
    p[10 + idx * 4] = c.g;
    p[11 + idx * 4] = c.b;
    p[30] = 0x64; // brightness
    p[31] = 0x04; // direct type
    p[36] = zone_value; // zone
    p[37] = 0x01; // powered on
    p
}
