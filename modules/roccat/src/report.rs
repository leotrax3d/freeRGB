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
pub fn direct_mode() -> Vec<u8> {
    vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]
}
pub fn keyboard_direct_mode(new_protocol: bool) -> Vec<u8> {
    if new_protocol {
        vec![0x0e, 0x05, 0x01, 0x00, 0x00]
    } else {
        vec![0x15, 0x00, 0x01]
    }
}
pub fn kova_init() -> Vec<u8> {
    vec![0x04, 0x00, 0x80]
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

pub fn burst_pro_air(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 30];
    p[0..5].copy_from_slice(&[0x0f, 0x01, 0x4c, 0x06, 0x14]);
    for (n, c) in colors.iter().enumerate().take(4) {
        let q = 5 + n * 5;
        p[q..q + 5].copy_from_slice(&[(n + 1) as u8, 0xff, c.r, c.g, c.b]);
    }
    p
}

pub fn kone_pro_air(colors: &[Rgb], direct: bool) -> Vec<u8> {
    let mut p = vec![0; 16];
    p[1..10].copy_from_slice(&[
        0x10,
        if direct { 0x10 } else { 0x50 },
        0x0b,
        if direct { 0 } else { 1 },
        0x0b,
        0xff,
        0xff,
        0xff,
        0x0b,
    ]);
    for (n, c) in colors.iter().enumerate().take(2) {
        let q = 10 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}

pub fn kova(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 28];
    p[0] = 6;
    p[1] = 28;
    p[14] = 0x33;
    for (idx, c) in colors.iter().enumerate().take(2) {
        let q = if idx == 0 { 22 } else { 18 };
        p[q..q + 4].copy_from_slice(&[0, c.r, c.g, c.b]);
    }
    let checksum: u16 = p[..26].iter().map(|x| u16::from(*x)).sum();
    p[26] = checksum as u8;
    p[27] = (checksum >> 8) as u8;
    p
}

pub fn keyboard(
    colors: &[Rgb],
    packet_length: usize,
    column_length: usize,
    protocol: u8,
    big_endian: bool,
) -> Vec<Vec<u8>> {
    let packet_count = packet_length.div_ceil(64);
    let header_length = if packet_length > 255 { 4 } else { 3 };
    let data_length = 64 - header_length;
    let mut packets = vec![vec![0; 65]; packet_count];
    for (index, packet) in packets.iter_mut().enumerate() {
        packet[1] = 0xa1;
        packet[2] = if protocol > 1 { (index + 1) as u8 } else { 1 };
    }
    if header_length == 3 {
        packets[0][3] = packet_length as u8;
    } else if big_endian {
        packets[0][3] = (packet_length >> 8) as u8;
        packets[0][4] = packet_length as u8;
    } else {
        packets[0][3] = packet_length as u8;
        packets[0][4] = (packet_length >> 8) as u8;
    }
    for (index, color) in colors.iter().enumerate() {
        let column = index / column_length;
        let row = index % column_length;
        let logical = column * 3 * column_length + row;
        for (channel, value) in [(0, color.r), (1, color.g), (2, color.b)] {
            let offset =
                logical + channel * column_length + if protocol == 1 { header_length } else { 0 };
            let (packet, byte) = if protocol == 1 {
                (offset / 64, offset % 64 + 1)
            } else {
                (
                    offset / data_length,
                    offset % data_length + header_length + 1,
                )
            };
            if packet < packets.len() && byte < 65 {
                packets[packet][byte] = value;
            }
        }
    }
    packets
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
    fn burst_air_and_kova_are_byte_exact() {
        let c = [Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }];
        assert_eq!(
            &burst_pro_air(&c)[..10],
            &[0x0f, 1, 0x4c, 6, 0x14, 1, 0xff, 1, 2, 3]
        );
        let k = kova(&c);
        assert_eq!(&k[18..26], &[0, 4, 5, 6, 0, 1, 2, 3]);
    }
    #[test]
    fn keyboard_checksum_is_little_endian() {
        let p = keyboard(&[Rgb { r: 1, g: 2, b: 3 }], 26, 5, 2, false);
        assert_eq!(p.len(), 1);
        assert_eq!(&p[0][1..4], &[0xa1, 1, 26]);
        assert_eq!((p[0][4], p[0][9], p[0][14]), (1, 2, 3));
    }
}
