// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn state(channel: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0x38;
    p[2] = channel;
    p[3] = 2;
    p
}
pub fn direct(channel: u8, start: u8, colors: &[Rgb], plane: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0x32;
    p[2] = channel;
    p[3] = start;
    p[4] = colors.len() as u8;
    p[5] = plane;
    for (i, c) in colors.iter().enumerate() {
        p[6 + i] = match plane {
            0 => c.r,
            1 => c.g,
            _ => c.b,
        };
    }
    p
}
pub fn commit() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0x33;
    p[2] = 0xff;
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    #[test]
    fn exact() {
        assert_eq!(&state(1)[0..4], &[0, 0x38, 1, 2]);
        assert_eq!(
            &direct(1, 0, &[Rgb { r: 1, g: 2, b: 3 }], 0)[0..7],
            &[0, 0x32, 1, 0, 1, 0, 1]
        );
        assert_eq!(&commit()[0..3], &[0, 0x33, 0xff]);
    }
}
