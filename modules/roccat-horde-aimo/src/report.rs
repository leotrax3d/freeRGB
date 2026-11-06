// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream constants from `RoccatHordeAimoController.h`.
pub const WRITE_PACKET_LENGTH: usize = 24;
pub const REPORT_ID: u8 = 0x18;
pub const LED_COUNT: usize = 6;

/// Upstream: RoccatHordeAimoController.cpp InitialPacket (controller
/// constructor): 8-byte feature report `[13 08 01]` zero padded.
pub fn init() -> Vec<u8> {
    let mut p = vec![0u8; 8];
    p[0] = 0x13;
    p[1] = 0x08;
    p[2] = 0x01;
    p
}

/// Upstream: RoccatHordeAimoController.cpp SetColors: 24-byte feature report
/// `[18 18 FF FF]` + six RGB triplets, then a truncated 16-bit checksum at
/// bytes 22/23 over bytes 0..21, followed by a 3-byte feature read.
pub fn set_colors(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; WRITE_PACKET_LENGTH];
    p[0] = REPORT_ID;
    p[1] = WRITE_PACKET_LENGTH as u8;
    p[2] = 0xff;
    p[3] = 0xff;
    for (i, c) in colors.iter().enumerate().take(LED_COUNT) {
        let b = 4 + i * 3;
        p[b] = c.r;
        p[b + 1] = c.g;
        p[b + 2] = c.b;
    }
    let mut crc: u32 = 0;
    for b in &p[..WRITE_PACKET_LENGTH - 2] {
        crc = crc.wrapping_add(u32::from(*b));
    }
    p[22] = crc as u8;
    p[23] = (crc >> 8) as u8;
    p
}
