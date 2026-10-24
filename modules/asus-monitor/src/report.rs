// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn query() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[0] = 0xec;
    p[1] = 0xb0;
    p
}
pub fn init() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[0] = 0xec;
    p[1] = 0x35;
    p[5] = 0xff;
    p[8] = 1;
    p
}
pub fn direct(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[0] = 0xec;
    p[1] = 0x40;
    p[2] = 0x84;
    p[4] = colors.len() as u8;
    for (i, c) in colors.iter().enumerate() {
        let q = 5 + i * 3;
        if q + 2 >= p.len() {
            break;
        }
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        assert_eq!(&query()[0..2], &[0xec, 0xb0]);
        assert_eq!(&init()[0..9], &[0xec, 0x35, 0, 0, 0, 0xff, 0, 0, 1]);
        assert_eq!(
            &direct(&[Rgb { r: 1, g: 2, b: 3 }])[0..9],
            &[0xec, 0x40, 0x84, 0, 1, 1, 2, 3, 0]
        );
    }
}
