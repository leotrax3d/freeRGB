// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(protocol: &str, colors: &[Rgb]) -> Vec<u8> {
    let (len, id, offset) = match protocol {
        "surge" => (264, 0x14, 8),
        "fps" => (264, 0x0a, 8),
        "raid" => (264, 0x0a, 9),
        "haste" => (65, 0x81, 8),
        _ => (65, 0xd2, 8),
    };
    let mut p = vec![0; len];
    p[1] = id;
    for (i, c) in colors.iter().enumerate() {
        let q = offset + i * 3;
        if q + 2 >= p.len() {
            break;
        }
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packets_are_exact() {
        let c = [Rgb { r: 1, g: 2, b: 3 }];
        assert_eq!(
            &direct("haste", &c)[0..12],
            &[0, 0x81, 0, 0, 0, 0, 0, 0, 1, 2, 3, 0]
        );
        assert_eq!(direct("surge", &c).len(), 264);
        assert_eq!(direct("dart", &c).len(), 65);
    }
}
