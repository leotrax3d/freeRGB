// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: CryorigH7QuadLumiController.cpp SetChannelLEDs / SendPacket
pub const REPORT_LEN: usize = 65;
pub fn direct(channel: usize, colors: &[Rgb; 5]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x02;
    p[1] = 0x4C;
    p[2] = (channel as u8 + 1) & !0x10; // direction clear
    p[3] = 0x00; // MODE_FIXED
    p[4] = 0x00; // color index 0 | speed 0
    for (i, c) in colors.iter().enumerate() {
        let off = 5 + (channel * 15) + (i * 3);
        p[off] = c.g;
        p[off + 1] = c.r;
        p[off + 2] = c.b;
    }
    p
}
