// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

pub fn direct_mouse(colors: &[Rgb], report_id: u8, byte: u8, len: usize) -> Vec<u8> {
    let mut p = vec![0; len];
    p[0] = report_id;
    p[1] = byte;
    for (n, c) in colors.iter().enumerate().take((len.saturating_sub(2)) / 3) {
        let q = 2 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn mouse(colors: &[Rgb], len: usize) -> Vec<u8> {
    let mut p = vec![0; len];
    p[0] = 0x0d;
    p[1] = 0x2e;
    for (n, c) in colors.iter().enumerate().take((len.saturating_sub(2)) / 4) {
        let q = 2 + n * 4;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn mat(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 9];
    p[0] = 3;
    for (n, c) in colors.iter().enumerate().take(2) {
        let q = 1 + n * 4;
        p[q..q + 4].copy_from_slice(&[c.r, c.g, c.b, 0xff]);
    }
    p
}
pub fn headset(c: Rgb) -> Vec<u8> {
    let mut p = vec![0; 64];
    p[0] = 0x0f;
    p[1] = 4;
    p[4..7].copy_from_slice(&[c.r, c.g, c.b]);
    p
}

pub fn keyboard(
    colors: &[Rgb],
    packet_length: usize,
    column_length: usize,
    protocol: u8,
    big_endian: bool,
) -> Vec<Vec<u8>> {
    let mut p = vec![0; packet_length];
    let header = if packet_length > 255 { 2 } else { 1 };
    p[0] = if protocol == 1 { 0x0d } else { 0x11 };
    if header == 1 {
        p[1] = packet_length as u8;
    } else if big_endian {
        p[1] = (packet_length >> 8) as u8;
        p[2] = packet_length as u8;
    } else {
        p[1] = packet_length as u8;
        p[2] = (packet_length >> 8) as u8;
    }
    let offset = header + 1;
    p[offset] = 0;
    p[offset + 1] = 0;
    p[offset + 2] = 0xff;
    p[offset + 3] = if protocol == 1 { 0 } else { 0xff };
    p[offset + 4] = 0;
    let channel = packet_length.saturating_sub(2) / 3;
    for (i, c) in colors.iter().enumerate() {
        let led = i / column_length * column_length + i % column_length;
        let q = offset + 6 + led;
        if q + 2 * channel < packet_length.saturating_sub(2) {
            p[q] = c.r;
            p[q + channel] = c.g;
            p[q + 2 * channel] = c.b;
        }
    }
    let checksum: u16 = p[..packet_length.saturating_sub(2)]
        .iter()
        .map(|x| u16::from(*x))
        .sum();
    if packet_length >= 2 {
        p[packet_length - 2] = checksum as u8;
        p[packet_length - 1] = (checksum >> 8) as u8;
    }
    p.chunks(64)
        .map(|chunk| {
            let mut r = vec![0; 65];
            r[1..1 + chunk.len()].copy_from_slice(chunk);
            r
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    #[test]
    fn direct_mouse_is_byte_exact() {
        assert_eq!(
            direct_mouse(&[Rgb { r: 1, g: 2, b: 3 }], 0x0d, 0x7a, 8),
            vec![0x0d, 0x7a, 1, 2, 3, 0, 0, 0]
        );
    }
    #[test]
    fn mat_is_byte_exact() {
        assert_eq!(&mat(&[Rgb { r: 1, g: 2, b: 3 }])[..5], &[3, 1, 2, 3, 0xff]);
    }
    #[test]
    fn keyboard_checksum_is_little_endian() {
        let p = keyboard(&[Rgb { r: 1, g: 2, b: 3 }], 26, 5, 2, false).concat();
        assert_eq!(&p[25..27], &[41, 2]);
    }
}
