// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn colors(channel: u8, colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (n, ch) in colors.chunks(20).enumerate() {
        let mut p = vec![0; 64];
        p[0] = channel;
        p[1] = n as u8;
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
    fn exact_64_byte_packets() {
        let p = colors(1, &[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(p.len(), 1);
        assert_eq!(&p[0][0..6], &[1, 0, 1, 2, 3, 0]);
        assert_eq!(p[0].len(), 64);
    }
}
