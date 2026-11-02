// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 65;
pub fn init() -> Vec<u8> {
    vec![0, 0x90, 0, 0, 0, 0, 0, 1, 0x6f]
}
pub fn frames(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut raw = vec![0u8; 64 * 8];
    for (n, c) in colors.iter().take(64).enumerate() {
        let q = n * 4;
        raw[q + 1..q + 4].copy_from_slice(&[c.r, c.g, c.b]);
    }
    (0..8)
        .map(|n| {
            let mut p = vec![0u8; REPORT_LEN];
            p[1..].copy_from_slice(&raw[n * 64..(n + 1) * 64]);
            p
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = frames(&[Rgb { r: 1, g: 2, b: 3 }; 1]);
        assert_eq!(p.len(), 8);
        assert_eq!(&p[0][..5], [0, 0, 1, 2, 3]);
        assert_eq!(p[7].len(), 65);
    }
}
