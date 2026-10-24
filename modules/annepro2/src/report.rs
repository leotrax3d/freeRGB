// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut raw = vec![0; 210];
    for (i, c) in colors.iter().take(61).enumerate() {
        raw[i * 3..i * 3 + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    let n = raw.len();
    let mut out = Vec::new();
    let max = 51;
    let count = n / max + 1;
    for pidx in 0..count {
        let start = pidx * max;
        let part = raw.get(start..(start + max).min(n)).unwrap_or(&[]);
        let mut p = vec![0; 64];
        p[0] = 0;
        p[1] = 0x7b;
        p[2] = 0x10;
        p[3] = 0x41;
        p[4] = ((count << 4) + pidx) as u8;
        p[5] = (part.len() + 4) as u8;
        p[6] = 0;
        p[7] = 0;
        p[8] = 0x7d;
        p[9] = 32;
        p[10] = 3;
        p[11] = 0xff;
        p[12] = 2;
        p[13..13 + part.len()].copy_from_slice(part);
        out.push(p)
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = direct(&[Rgb { r: 1, g: 2, b: 3 }; 61]);
        assert_eq!(p.len(), 5);
        assert_eq!(
            &p[0][0..14],
            &[0, 0x7b, 0x10, 0x41, 0x50, 55, 0, 0, 0x7d, 32, 3, 0xff, 2, 1]
        );
        assert_eq!(p[0].len(), 64);
    }
}
