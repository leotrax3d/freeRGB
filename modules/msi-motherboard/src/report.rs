// SPDX-License-Identifier: GPL-2.0-or-later
// Byte layouts mirror MSIMotherboardCommon.h from the pinned OpenRGB source.

pub const REPORT_112: usize = 112;
pub const REPORT_162: usize = 162;
pub const REPORT_185: usize = 185;
pub const REPORT_761: usize = 761;
pub const REPORT_CONFIG: usize = 290;
pub const ZONE_SIZE: usize = 10;

fn zone(dst: &mut [u8], offset: usize, r: u8, g: u8, b: u8, count: usize) {
    dst[offset] = 1; // MSI_MODE_STATIC
    dst[offset + 1..offset + 4].copy_from_slice(&[r, g, b]);
    dst[offset + 4] = 40; // 100% brightness, medium speed
    dst[offset + 5..offset + 8].copy_from_slice(&[r, g, b]);
    dst[offset + 8] = 0x80; // custom colour
    if count > 1 {
        dst[offset + 9] = 0;
    }
}

pub fn zone_packet(len: usize, colors: &[freergb_core::Rgb], zones: &[usize]) -> Vec<u8> {
    let mut out = vec![0; len];
    out[0] = 0x52;
    let mut cursor = 0;
    for (idx, &count) in zones.iter().enumerate() {
        let c = colors.get(cursor).copied().unwrap_or_default();
        let offset = match len {
            REPORT_112 => [1, 11, 21, 31, 41, 51, 61, 71, 81, 91, 101][idx],
            REPORT_162 => [
                1, 11, 21, 31, 41, 51, 61, 71, 81, 91, 101, 111, 121, 131, 141, 151,
            ][idx],
            _ => [
                1, 11, 21, 31, 42, 53, 64, 74, 84, 94, 104, 114, 124, 134, 144, 154, 164, 174,
            ][idx],
        };
        zone(&mut out, offset, c.r, c.g, c.b, count);
        cursor += count;
    }
    out[len - 1] = 0;
    out
}

pub fn per_led_761(
    zone_header: u8,
    zone_index: u8,
    colors: &[freergb_core::Rgb],
    leds: usize,
) -> Vec<u8> {
    let mut out = vec![0; REPORT_761];
    out[0] = 0x51;
    out[1] = 0x09;
    out[2] = zone_header;
    out[3] = zone_index;
    out[6] = leds.min(240) as u8;
    for (i, c) in colors.iter().take(240).enumerate() {
        let p = 7 + i * 3;
        out[p..p + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    out
}

pub fn config_761(zone_counts: &[u8]) -> Vec<u8> {
    let mut out = vec![0; REPORT_CONFIG];
    out[0] = 0x50;
    for (row, count) in zone_counts.iter().take(18).enumerate() {
        out[row * 16 + 16] = *count;
    }
    out
}
