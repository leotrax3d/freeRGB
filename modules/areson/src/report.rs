// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: AresonController.cpp AresonController::SetMode driven by the Static
/// direct mode of RGBController_Areson.cpp (mode STATIC_MODE_VALUE = 0x02,
/// speed n/a -> 0, brightness max -> 0xFF).
pub const DIRECT_LENGTH: usize = 17;
pub fn direct(c: Rgb) -> Vec<u8> {
    let mut p = vec![0; DIRECT_LENGTH];
    p[0] = 0x08;
    p[1] = 7;
    p[4] = 0xa0;
    p[5] = 7;
    p[6] = 0x02;
    p[7] = c.r;
    p[8] = c.g;
    p[9] = c.b;
    p[10] = 0;
    p[11] = 0xff;
    p[12] = 0x55u8
        .wrapping_sub(p[6])
        .wrapping_sub(p[7])
        .wrapping_sub(p[8])
        .wrapping_sub(p[9])
        .wrapping_sub(p[10])
        .wrapping_sub(p[11]);
    p[16] = 0x4a;
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        assert_eq!(
            &direct(Rgb { r: 1, g: 2, b: 3 })[0..13],
            &[8, 7, 0, 0, 0xa0, 7, 0x02, 1, 2, 3, 0, 0xff, 0x4E]
        );
        assert_eq!(direct(Rgb { r: 0, g: 0, b: 0 })[16], 0x4a);
    }
}
