// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

pub const REPORT_LEN: usize = 65;
pub const CHUNK_SIZE: usize = 15;
pub const LED_COUNT: usize = 91;

pub const LED_IDS: [u8; LED_COUNT] = [
    0x01, 0x04, 0x07, 0x0A, 0x0D, 0x10, 0x13, 0x16, 0x19, 0x1C, 0x1F, 0x22, 0x25, 0x28, 0x2B, 0x2E,
    0x31, 0x34, 0x37, 0x3A, 0x3D, 0x40, 0x43, 0x46, 0x49, 0x4C, 0x4F, 0x52, 0x55, 0x58, 0x5B, 0x5E,
    0x02, 0x05, 0x08, 0x0B, 0x0E, 0x11, 0x14, 0x17, 0x00, 0x03, 0x06, 0x09, 0x0C, 0x0F, 0x12, 0x15,
    0x18, 0x1B, 0x1E, 0x21, 0x24, 0x27, 0x2A, 0x2D, 0x30, 0x33, 0x36, 0x39, 0x3C, 0x3F, 0x42, 0x45,
    0x48, 0x4B, 0x4E, 0x51, 0x54, 0x57, 0x5A, 0x5D, 0x60, 0x63, 0x66, 0x69, 0x6C, 0x6F, 0x72, 0x75,
    0x78, 0x7B, 0x7E, 0x81, 0x84, 0x87, 0x8A, 0x8D, 0x90, 0x93, 0x96,
];

pub fn firmware_query() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x12;
    p[2] = 0x20;
    p
}

pub fn parse_firmware(response: &[u8]) -> String {
    let mut bytes = Vec::new();
    for k in 0..8 {
        let b = response.get(8 + k * 2).copied().unwrap_or(0);
        if b == 0 {
            break;
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn enable() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x41;
    p[2] = 0x03;
    p
}

pub fn apply() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[1] = 0x51;
    p[2] = 0x28;
    p
}

pub fn frames(colors: &[Rgb]) -> Vec<[u8; REPORT_LEN]> {
    let mut packets = Vec::new();
    for (chunk_idx, chunk) in colors.chunks(CHUNK_SIZE).enumerate() {
        let base = chunk_idx * CHUNK_SIZE;
        let mut packet = [0u8; REPORT_LEN];
        packet[1] = 0xC0;
        packet[2] = 0x01;
        packet[3] = chunk.len() as u8;
        packet[4] = 0x00;
        for (j, color) in chunk.iter().enumerate() {
            let led_id = LED_IDS[base + j];
            let (r, g, b) = if led_id == 0x96 {
                (color.r.max(color.g).max(color.b), 0u8, 0u8)
            } else {
                (color.r, color.g, color.b)
            };
            let p = 5 + j * 4;
            packet[p] = led_id;
            packet[p + 1] = r;
            packet[p + 2] = g;
            packet[p + 3] = b;
        }
        packets.push(packet);
    }
    packets
}
