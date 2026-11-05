// SPDX-License-Identifier: GPL-2.0-or-later
/// upstream: ClevoLightbarController.cpp SetColor/SetMode
pub const REPORT_LEN: usize = 8;
pub fn color(c: freergb_core::Rgb) -> [u8; REPORT_LEN] {
    [0x14, 0x00, 0x01, c.r, c.g, c.b, 0x00, 0x00]
}
pub fn mode_direct() -> [u8; REPORT_LEN] {
    // mode=0x01, inverted speed = (10+1)-5 = 6, brightness=100, trailing 0x01
    [0x08, 0x22, 0x01, 0x06, 0x64, 0x01, 0x00, 0x00]
}