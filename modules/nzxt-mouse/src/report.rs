// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn firmware_request() -> [u8; 64] {
    let mut b = [0u8; 64];
    b[0] = 0x43;
    b[1] = 0x81;
    b[3] = 0x01;
    b
}
pub fn direct(colors: &[Rgb]) -> [u8; 64] {
    let mut b = [0u8; 64];
    b[0] = 0x43;
    b[1] = 0xAE;
    b[3] = 0x10;
    b[4] = 0x02;
    b[5] = 0x3F;
    b[24] = 0x06;
    if colors.len() > 2 {
        let c = colors[2];
        b[25] = c.r;
        b[26] = c.g;
        b[27] = c.b;
    }
    if colors.len() > 1 {
        let c = colors[1];
        b[29] = c.r;
        b[30] = c.g;
        b[31] = c.b;
    }
    if !colors.is_empty() {
        let c = colors[0];
        b[33] = c.r;
        b[34] = c.g;
        b[35] = c.b;
    }
    if colors.len() > 3 {
        let c = colors[3];
        b[37] = c.r;
        b[38] = c.g;
        b[39] = c.b;
    }
    if colors.len() > 4 {
        let c = colors[4];
        b[41] = c.r;
        b[42] = c.g;
        b[43] = c.b;
    }
    if colors.len() > 5 {
        let c = colors[5];
        b[45] = c.r;
        b[46] = c.g;
        b[47] = c.b;
    }
    b
}
