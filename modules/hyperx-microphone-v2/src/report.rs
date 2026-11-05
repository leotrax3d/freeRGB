// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: HyperXMicrophoneV2Controller.cpp SendDirect
pub const REPORT_LEN: usize = 64;
pub fn header() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x44;
    p[1] = 0x01;
    p[2] = 0x06;
    p
}
pub fn color_packet(packet_index: usize, colors: &[Rgb]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x44;
    p[1] = 0x02;
    p[2] = packet_index as u8;
    p[3] = 0x00;
    for (i, c) in colors.iter().enumerate() {
        let off = 4 + i * 3;
        p[off] = c.r;
        p[off + 1] = c.g;
        p[off + 2] = c.b;
    }
    p
}
