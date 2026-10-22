// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn packet(channel: u8, colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 97];
    p[0] = channel;
    p[1] = 0x22;
    for (i, c) in colors.iter().take(31).enumerate() {
        let q = 2 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = packet(1, &[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(p.len(), 97);
        assert_eq!(&p[0..6], &[1, 0x22, 1, 2, 3, 0]);
    }
}
