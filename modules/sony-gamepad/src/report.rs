// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn packet(colors: &[Rgb], dualsense: bool) -> Vec<u8> {
    if !dualsense {
        let c = colors[0];
        return vec![5, 7, 0, 0, 0, 0, c.r, c.g, c.b, 0, 0];
    }
    let mut p = vec![0u8; 48];
    p[0] = 2;
    p[1] = 0x0f;
    p[2] = 0x55;
    p[39] = 0xff;
    p[43] = 2;
    p[44] = 0x20;
    for (n, c) in colors.iter().skip(1).take(5).enumerate() {
        if c.r != 0 || c.g != 0 || c.b != 0 {
            p[44] |= 1 << n;
        }
    }
    let c = colors[0];
    p[45..48].copy_from_slice(&[c.r, c.g, c.b]);
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ds4_usb_exact() {
        assert_eq!(
            packet(&[Rgb { r: 1, g: 2, b: 3 }], false),
            vec![5, 7, 0, 0, 0, 0, 1, 2, 3, 0, 0]
        );
    }
    #[test]
    fn dualsense_usb_exact() {
        let mut x = vec![Rgb { r: 1, g: 2, b: 3 }; 6];
        x[2] = Rgb { r: 0, g: 0, b: 0 };
        let p = packet(&x, true);
        assert_eq!(&p[..3], [2, 0x0f, 0x55]);
        assert_eq!(&p[44..48], [0x3d, 1, 2, 3]);
    }
}
