// SPDX-License-Identifier: GPL-2.0-or-later
use crate::keys::KEYS;
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 65;
pub fn init_packets() -> [Vec<u8>; 3] {
    let mut a = vec![0u8; REPORT_LEN];
    a[0] = 0;
    a[1] = 0x08;
    a[2] = 0x01;
    a[3] = 0x03;
    a[5] = 0x02;
    let mut b = vec![0u8; REPORT_LEN];
    b[0] = 0;
    b[1] = 0x08;
    b[2] = 0x02;
    b[3] = 0x5F;
    let mut c = vec![0u8; REPORT_LEN];
    c[0] = 0;
    c[1] = 0x08;
    c[2] = 0x0D;
    c[3] = 0x00;
    c[4] = 0x01;
    [a, b, c]
}
pub fn frames(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut bank = [0u8; 137 * 3];
    for (i, color) in colors.iter().enumerate() {
        if i < KEYS.len() {
            let offset = KEYS[i];
            bank[offset] = color.r;
            bank[137 + offset] = color.g;
            bank[274 + offset] = color.b;
        }
    }
    let mut packets = Vec::new();
    // first packet 53 payload
    let mut p = vec![0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x08;
    p[2] = 0x06;
    p[3] = 0x00;
    p[4] = 0x9B;
    p[5] = 0x01;
    p[12..65].copy_from_slice(&bank[..53]);
    packets.push(p);
    // continuation
    let mut color_ptr = 53usize;
    for _ in 0..6usize {
        let mut p2 = vec![0u8; REPORT_LEN];
        p2[0] = 0;
        p2[1] = 0x08;
        p2[2] = 0x07;
        p2[3] = 0x00;
        let end = (color_ptr + 61).min(bank.len());
        let len = end - color_ptr;
        p2[4..4 + len].copy_from_slice(&bank[color_ptr..end]);
        color_ptr += 61;
        packets.push(p2);
    }
    packets
}
