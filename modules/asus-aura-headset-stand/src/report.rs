// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

pub fn frame(colors: &[Rgb]) -> Vec<Vec<u8>> {
    // 15 LEDs in report 0, and 4 (LEDs 15..18) in report 1.
    let mut r0 = vec![0u8; 65];
    r0[0] = 0x00;
    r0[1] = 0xc0;
    r0[2] = 0x81;
    r0[3] = 0x00;
    r0[4] = 0x00;
    for i in (0..60).step_by(4) {
        r0[5 + i] = 0x00;
        let which = i / 4;
        r0[6 + i] = colors[which].r;
        r0[7 + i] = colors[which].g;
        r0[8 + i] = colors[which].b;
    }
    let mut r1 = vec![0u8; 65];
    r1[0] = 0x00;
    r1[1] = 0xc0;
    r1[2] = 0x81;
    r1[3] = 0x01;
    r1[4] = 0x00;
    for i in (0..12).step_by(4) {
        r1[5 + i] = 0x00;
        let which = (i / 4) + 15;
        r1[6 + i] = colors[which].r;
        r1[7 + i] = colors[which].g;
        r1[8 + i] = colors[which].b;
    }
    vec![r0, r1]
}
