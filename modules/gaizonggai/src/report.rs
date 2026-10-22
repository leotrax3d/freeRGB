// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn packets(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (n, ch) in colors.chunks(21).enumerate() {
        let mut p = vec![0; 65];
        p[1] = n as u8 + 0x10;
        for (i, c) in ch.iter().enumerate() {
            let q = 2 + i * 3;
            p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
        }
        out.push(p)
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let c = [Rgb { r: 1, g: 2, b: 3 }];
        assert_eq!(&packets(&c)[0][0..6], &[0, 0x10, 1, 2, 3, 0]);
        assert_eq!(packets(&[Rgb { r: 0, g: 0, b: 0 }; 22]).len(), 2);
    }
}
