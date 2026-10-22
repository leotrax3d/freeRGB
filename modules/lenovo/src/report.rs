// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn legacy(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (chunk, group) in colors.chunks(47).enumerate() {
        let mut p = vec![0; 192];
        p[0] = 0x07;
        p[1] = 0xa0;
        p[2] = group.len() as u8;
        for (i, c) in group.iter().enumerate() {
            let q = 4 + i * 4;
            p[q] = i as u8 + (chunk * 47) as u8;
            p[q + 1..q + 4].copy_from_slice(&[c.r, c.g, c.b]);
        }
        out.push(p);
    }
    out
}
pub fn gen7(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 960];
    p[0] = 0x07;
    p[1] = 0xa1;
    p[2] = 0xc0;
    p[3] = 0x03;
    for (i, c) in colors.iter().enumerate().take((960 - 4) / 5) {
        let q = 4 + i * 5;
        p[q] = i as u8;
        p[q + 1] = (i >> 8) as u8;
        p[q + 2..q + 5].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn four_zone(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 33];
    p[0] = 4;
    p[1] = 1;
    for (i, c) in colors.iter().take(4).enumerate() {
        let q = 5 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn m300(c: Rgb) -> Vec<u8> {
    let mut p = vec![0; 0x41];
    p[1] = 0x25;
    p[3] = 1;
    p[4] = 1;
    p[5] = 0x0c;
    p[6] = 3;
    p[7] = 0x64;
    p[9..12].copy_from_slice(&[c.r, c.g, c.b]);
    p[0x40] = p[..0x0c].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    p
}
pub fn k510() -> Vec<u8> {
    let mut p = vec![0; 64];
    p[0] = 4;
    p[3] = 6;
    p[4] = 0x38;
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_header_is_exact() {
        let p = legacy(&[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(&p[0][..8], &[7, 0xa0, 1, 0, 0, 1, 2, 3]);
    }
    #[test]
    fn gen7_header_is_exact() {
        let p = gen7(&[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(&p[..9], &[7, 0xa1, 0xc0, 3, 0, 0, 1, 2, 3]);
    }
    #[test]
    fn m300_checksum_is_exact() {
        let p = m300(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(
            p[0x40],
            p[..0x0c].iter().fold(0u8, |a, b| a.wrapping_add(*b))
        );
    }
}
