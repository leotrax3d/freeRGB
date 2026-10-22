// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn legacy(pid: u16, colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let groups: &[(u8, usize)] = match pid {
        0xc968 => &[(1, 145), (2, 13), (3, 105), (4, 99)],
        0xc935 => &[(1, 108)],
        0xc956 => &[(1, 148)],
        0xc957 | 0xc967 => &[(1, 145)],
        _ => &[(1, colors.len())],
    };
    let mut offset = 0;
    for (zone, count) in groups {
        for group in colors[offset..(offset + count).min(colors.len())].chunks(47) {
            let mut p = vec![0; 192];
            p[0] = 0x07;
            p[1] = 0xa0 + zone;
            p[2] = group.len() as u8;
            let base = offset;
            for (i, c) in group.iter().enumerate() {
                let q = 4 + i * 4;
                p[q] = (base + i) as u8;
                p[q + 1..q + 4].copy_from_slice(&[c.r, c.g, c.b]);
            }
            out.push(p);
        }
        offset += *count;
        if offset >= colors.len() {
            break;
        }
    }
    out
}
fn gen7_ids(pid: u16, count: usize) -> Vec<u16> {
    let mut ids: Vec<u16> = (1..=101).collect();
    match pid {
        0xc978 => {
            ids.push(0xdd);
            ids.extend(0xe9..=0xf0);
            ids.extend(0xf5..=0xfe);
        }
        0xc197 => ids.extend(0xe9..=0xfa),
        0xc195 => ids = (1..=24).collect(),
        0xc977 | 0xc987 => {}
        _ => ids.extend(0xf5..=0xfe),
    }
    ids.truncate(count);
    ids
}
pub fn gen7(pid: u16, colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 960];
    p[0] = 0x07;
    p[1] = 0xa1;
    p[2] = 0xc0;
    p[3] = 0x03;
    for (i, (id, c)) in gen7_ids(pid, colors.len())
        .into_iter()
        .zip(colors.iter())
        .enumerate()
    {
        let q = 4 + i * 5;
        p[q] = id as u8;
        p[q + 1] = (id >> 8) as u8;
        p[q + 2..q + 5].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn four_zone(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 33];
    p[0] = 0xcc;
    p[1] = 0x16;
    p[2] = 1;
    p[3] = 1;
    p[4] = 1;
    for (i, c) in colors.iter().take(4).enumerate() {
        let q = 5 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn m300(c: Rgb) -> Vec<u8> {
    let mut p = vec![0; 0x41];
    p[1] = 0x25;
    p[3] = 1;
    p[4] = 1;
    p[5] = 0x0c;
    p[6] = 3;
    p[7] = 0x64;
    p[9..12].copy_from_slice(&[c.r, c.g, c.b]);
    p[0x40] = p[..0x0c].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    p
}
pub fn k510() -> Vec<u8> {
    vec![]
}
pub fn k500(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut feature = vec![0; 9];
    feature[1] = 0x14;
    feature[3] = 0x5a;
    let mut p = vec![0; 65];
    p[1] = 1;
    p[2] = 0x08;
    p[3] = 4;
    p[4] = 2;
    if let Some(c) = colors.first() {
        p[0x11..0x14].copy_from_slice(&[c.r, c.g, c.b]);
    }
    if colors.len() > 1 {
        let c = colors[1];
        p[0x15..0x18].copy_from_slice(&[c.r, c.g, c.b]);
    }
    if colors.len() > 2 {
        let c = colors[2];
        p[0x19..0x1c].copy_from_slice(&[c.r, c.g, c.b]);
    }
    vec![feature, p]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_header_is_exact() {
        let p = legacy(0xc956, &[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(&p[0][..8], &[7, 0xa1, 1, 0, 0, 1, 2, 3]);
    }
    #[test]
    fn gen7_header_is_exact() {
        let p = gen7(0xc195, &[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(&p[..9], &[7, 0xa1, 0xc0, 3, 1, 0, 1, 2, 3]);
    }
    #[test]
    fn m300_checksum_is_exact() {
        let p = m300(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(
            p[0x40],
            p[..0x0c].iter().fold(0u8, |a, b| a.wrapping_add(*b))
        );
    }
}
