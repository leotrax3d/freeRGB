// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET_SIZE: usize = 64;
pub const MAX_DATA: usize = 0x36;
fn checksum(p: &mut [u8; PACKET_SIZE]) {
    let sum: u16 = p[3..].iter().map(|x| u16::from(*x)).sum();
    p[1] = sum as u8;
    p[2] = (sum >> 8) as u8;
}
pub fn color_packet(data: &[u8], offset: usize) -> [u8; PACKET_SIZE] {
    assert!(data.len() <= MAX_DATA);
    let mut p = [0; PACKET_SIZE];
    p[0] = 4;
    p[3] = 0x0b;
    p[4] = data.len() as u8;
    p[5] = offset as u8;
    p[6] = (offset >> 8) as u8;
    p[8..8 + data.len()].copy_from_slice(data);
    checksum(&mut p);
    p
}
pub fn colors(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut raw = Vec::with_capacity(colors.len() * 3);
    for c in colors {
        raw.extend_from_slice(&[c.r, c.g, c.b]);
    }
    raw.chunks(MAX_DATA)
        .enumerate()
        .map(|(n, d)| color_packet(d, n * MAX_DATA).to_vec())
        .collect()
}
pub fn mode(mode: u8, brightness: u8, speed: u8, random: bool, color: Rgb) -> Vec<u8> {
    let mut p = [0; PACKET_SIZE];
    p[0] = 4;
    p[3] = 6;
    p[4] = 9;
    p[5] = 0;
    p[7] = 0x55;
    p[8..17].copy_from_slice(&[
        0,
        mode,
        brightness,
        speed,
        0,
        random as u8,
        color.r,
        color.g,
        color.b,
    ]);
    checksum(&mut p);
    p.to_vec()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_report_is_byte_exact() {
        let p = color_packet(&[1, 2, 3], 0);
        assert_eq!(&p[..12], &[4, 0x14, 0, 0x0b, 3, 0, 0, 0, 1, 2, 3, 0]);
    }
    #[test]
    fn color_frames_chunk_at_54_bytes() {
        let c = vec![Rgb { r: 1, g: 2, b: 3 }; 125];
        let p = colors(&c);
        assert_eq!(p.len(), 7);
        assert_eq!(p[0].len(), 64);
        assert_eq!(p[6][4], 51);
        assert_eq!(p[6][5], 0x44);
        assert_eq!(p[6][6], 0x01);
    }
    #[test]
    fn mode_report_is_byte_exact() {
        let p = mode(3, 4, 2, true, Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(
            &p[0..17],
            &[4, 0x74, 0, 6, 9, 0, 0, 0x55, 0, 3, 4, 2, 0, 1, 1, 2, 3]
        );
    }
}
