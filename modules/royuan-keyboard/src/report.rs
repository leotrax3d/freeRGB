// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn frames(colors: &[Rgb], protocol: crate::table::Protocol) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if protocol == crate::table::Protocol::Skyloong {
        let mut raw = vec![0u8; 128 * 3];
        for (i, c) in colors.iter().enumerate() {
            let j = i * 3;
            if j + 2 < raw.len() {
                raw[j..j + 3].copy_from_slice(&[c.r, c.g, c.b]);
            }
        }
        for frame in 0usize..7 {
            let size: usize = if frame == 6 { 48 } else { 56 };
            let mut p = vec![0u8; 65];
            p[1] = 0x0c;
            p[2] = 0;
            p[3] = 0xff;
            p[4] = frame as u8;
            p[5] = if frame == 6 { 42 } else { 56 };
            p[6] = if frame == 6 { 1 } else { 0 };
            p[8] = checksum(&p[1..8]);
            let start = frame * 56;
            p[9..9 + size].copy_from_slice(&raw[start..start + size]);
            out.push(p);
        }
    }
    let mut p = vec![0u8; 65];
    p[1] = 0x07;
    p[2] = 1;
    p[3] = 0;
    p[4] = 4;
    p[5] = if protocol == crate::table::Protocol::Legacy {
        7
    } else {
        0
    };
    let c = colors.first().copied().unwrap_or(Rgb { r: 0, g: 0, b: 0 });
    p[6..9].copy_from_slice(&[c.r, c.g, c.b]);
    p[9] = checksum(&p[1..9]);
    out.push(p);
    out
}
fn checksum(data: &[u8]) -> u8 {
    let sum: u16 = data.iter().map(|x| *x as u16).sum();
    let mut n = 1u16;
    while n < sum {
        n <<= 1;
    }
    ((n - sum - 1) & 0xff) as u8
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = frames(&[Rgb { r: 1, g: 2, b: 3 }; 1], crate::table::Protocol::Akko);
        assert_eq!(p[0][..10], [0, 7, 1, 0, 4, 0, 1, 2, 3, 13]);
    }
}
