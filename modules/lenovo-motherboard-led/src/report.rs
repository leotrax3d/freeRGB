// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: LenovoMotherboardController.cpp SetMode (Static mode, mapped as Direct)
pub const REPORT_LEN: usize = 64;
pub fn direct(zone_value: u8, c: Rgb) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0xCC;
    p[1] = zone_value;
    p[2] = 0x01; // STATIC_MODE
    p[3] = 0x01; // speed min
    p[4] = 0x04; // brightness max
    p[5] = c.r;
    p[6] = c.g;
    p[7] = c.b;
    p[33] = zone_value;
    p[34] = 0x01;
    p[35] = 0x01;
    p[36] = 0x04;
    p[37] = c.r;
    p[38] = c.g;
    p[39] = c.b;
    p
}
