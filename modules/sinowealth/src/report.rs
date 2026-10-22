// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

pub fn zet(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 59];
    p[0] = 4;
    p[21] = 0x60;
    p[22] = 0;
    for (i, c) in colors.iter().take(7).enumerate() {
        let q = 23 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}

pub fn xenon(color: Rgb) -> Vec<u8> {
    let mut p = vec![0; 154];
    p[0] = 4;
    p[0x5d] = 0x20;
    p[0x60] = 1;
    p[0x61..0x64].copy_from_slice(&[color.r, color.g, color.b]);
    p
}

pub fn k10c(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 520];
    p[0] = 6;
    p[1] = 8;
    p[4] = 1;
    p[6] = 0x7a;
    p[7] = 1;
    for (i, c) in colors.iter().enumerate() {
        let q = 8 + i * 3;
        if q + 2 >= p.len() {
            break;
        }
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}

pub fn k16(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 1032];
    p[0] = 6;
    p[1] = 9;
    for (i, c) in colors.iter().take(132).enumerate() {
        p[8 + i] = c.b;
        p[8 + 132 + i] = c.g;
        p[8 + 264 + i] = c.r;
    }
    p
}

pub fn keyboard(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 1032];
    p[0] = 6;
    p[1] = 9;
    p[2] = 0xbc;
    p[4] = 0x40;
    for (i, c) in colors.iter().take(86).enumerate() {
        p[8 + i] = c.b;
        p[8 + 0x7e + i] = c.g;
        p[8 + 0xFC + i] = c.r;
    }
    p
}

pub fn mouse(colors: &[Rgb], sequence: u8) -> Vec<u8> {
    let mut p = vec![0; 64];
    p[3] = 2;
    p[4] = sequence;
    p[5] = 1;
    for (i, c) in colors.iter().take(1).enumerate() {
        let q = 8 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_match_openrgb_headers() {
        let c = [Rgb { r: 1, g: 2, b: 3 }];
        assert_eq!(
            &zet(&c)[0..26],
            &[4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x60, 0, 1, 2, 3]
        );
        assert_eq!(&xenon(c[0])[0..4], &[4, 0, 0, 0]);
        assert_eq!(&k10c(&c)[0..11], &[6, 8, 0, 0, 1, 0, 0x7a, 1, 1, 2, 3]);
        assert_eq!(&k16(&c)[0..12], &[6, 9, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0]);
        assert_eq!(
            &keyboard(&c)[0..12],
            &[6, 9, 0xbc, 0, 0x40, 0, 0, 0, 3, 0, 0, 0]
        );
        assert_eq!(&mouse(&c, 1)[0..11], &[0, 0, 0, 2, 1, 1, 0, 0, 1, 2, 3]);
    }
}
