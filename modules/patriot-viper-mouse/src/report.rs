// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET_LEN: usize = 64;
pub fn init_packet() -> [u8; PACKET_LEN] {
    [
        0x01, 0x00, 0x12, 0x12, 0x00, 0x00, 0x00, 0x00, 0x70, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0x50, 0xDE, 0x8D, 0x77, 0x09, 0xDF, 0x8D, 0x77, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x30, 0x58, 0x7C, 0x77, 0x78, 0x81, 0x43, 0x00, 0x30, 0x58, 0x7C, 0x77, 0x8C, 0x5D, 0x9B,
        0x77, 0x00, 0x00, 0x3D, 0x00, 0x98, 0xF5, 0x19, 0x08, 0x00, 0x00, 0x00, 0xEE,
    ]
}
fn checksum(buffer: &[u8; PACKET_LEN]) -> u8 {
    let xor_value = buffer[..63].iter().fold(0u8, |acc, &b| acc ^ b);
    if xor_value % 2 == 0 {
        ((xor_value as usize + 1) % 256) as u8
    } else {
        ((xor_value as usize + 255) % 256) as u8
    }
}
pub fn frames(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for led_index in 0u8..7u8 {
        let mut buffer = [0u8; PACKET_LEN];
        buffer[0] = 0x01;
        buffer[1] = 0x13;
        buffer[2] = led_index;
        buffer[3] = 0xFF;
        if (led_index as usize) < colors.len() {
            buffer[4] = colors[led_index as usize].r;
            buffer[5] = colors[led_index as usize].g;
            buffer[6] = colors[led_index as usize].b;
        }
        buffer[63] = checksum(&buffer);
        out.push(buffer.to_vec());
    }
    out
}