// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn mouse(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 64];
    p[0] = 7;
    p[1] = 3;
    p[2] = 6;
    p[3] = 2;
    for (i, c) in colors.iter().take(3).enumerate() {
        let q = 6 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn b820(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for plane in 0..3 {
        for half in 0..2 {
            let mut p: Vec<u8> = vec![0; 64];
            p[0] = 7;
            p[1] = 3;
            p[2] = 6;
            p[3] = (7 + plane * 2 + half) as u8;
            for (i, c) in colors.iter().skip(half * 58).take(58).enumerate() {
                p[6 + i] = match plane {
                    0 => c.r,
                    1 => c.g,
                    _ => c.b,
                };
            }
            out.push(p)
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let c = [Rgb { r: 1, g: 2, b: 3 }];
        assert_eq!(&mouse(&c)[0..10], &[7, 3, 6, 2, 0, 0, 1, 2, 3, 0]);
        assert_eq!(b820(&[Rgb { r: 1, g: 2, b: 3 }; 104]).len(), 6);
    }
}
