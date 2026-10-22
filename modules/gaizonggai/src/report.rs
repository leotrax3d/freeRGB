// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn packets(pid: u16, colors: &[Rgb]) -> Vec<Vec<u8>> {
    let (start, count) = match pid {
        0x4770 | 0x4771 | 0x4772 => (68, 20),
        0x4720 => (85, 21),
        0x4710..=0x4712 => (0, 20),
        _ => (0, 21),
    };
    let source = colors.get(start..).unwrap_or(&[]);
    let mut out = Vec::new();
    for (n, ch) in source.chunks(count).enumerate() {
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
        assert_eq!(&packets(0x4700, &c)[0][0..6], &[0, 0x10, 1, 2, 3, 0]);
        assert_eq!(packets(0x4700, &[Rgb { r: 0, g: 0, b: 0 }; 22]).len(), 2);
    }
}
