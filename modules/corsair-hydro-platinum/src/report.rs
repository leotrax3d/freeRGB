// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
fn pec(data: &[u8]) -> u8 {
    let mut x = 0;
    for b in data {
        x ^= *b;
        for _ in 0..8 {
            x = if x & 0x80 != 0 {
                (x << 1) ^ 0x07
            } else {
                x << 1
            };
        }
    }
    x
}
pub fn colors(colors: &[Rgb], command: u8, sequence: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0x3f;
    p[2] = (sequence << 3) | command;
    for (c, i) in colors.iter().take(20).zip(0..) {
        let q = 3 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.b, c.g, c.r]);
    }
    p[64] = pec(&p[2..64]);
    p
}
pub fn magic(magic: &[u8], command: u8, sequence: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0x3f;
    p[2] = (sequence << 3) | command;
    let n = magic.len().min(61);
    p[3..3 + n].copy_from_slice(&magic[..n]);
    p[64] = pec(&p[2..64]);
    p
}
#[cfg(test)]
pub(crate) fn pec_bytes(data: &[u8]) -> u8 {
    pec(data)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_packet_is_byte_exact() {
        let p = colors(&[Rgb { r: 1, g: 2, b: 3 }], 4, 1);
        assert_eq!(&p[..7], &[0, 0x3f, 0x0c, 3, 2, 1, 0]);
        assert_ne!(p[64], 0);
    }
    #[test]
    fn magic_packet_has_crc() {
        let p = magic(&[1, 2, 3], 1, 1);
        assert_eq!(&p[..7], &[0, 0x3f, 9, 1, 2, 3, 0]);
    }
}
