// SPDX-License-Identifier: GPL-2.0-or-later
use crate::packet_map::PACKET_MAP;
use freergb_core::Rgb;

pub const REPORT_LEN: usize = 256;
pub fn frames(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut rg = vec![0u8; REPORT_LEN];
    let mut ba = vec![0u8; REPORT_LEN];
    rg[0] = 0x08;
    rg[1] = 0x07;
    ba[0] = 0x08;
    ba[1] = 0x07;
    ba[3] = 0x01;
    for (i, color) in colors.iter().enumerate() {
        if i >= PACKET_MAP.len() {
            break;
        }
        let offset = PACKET_MAP[i];
        rg[5 + offset] = color.r;
        rg[107 + offset] = color.g;
        ba[5 + offset] = color.b;
    }
    vec![rg, ba]
}
