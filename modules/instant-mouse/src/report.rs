// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn frames(c: Rgb) -> Vec<Vec<u8>> {
    let r = 0x0f - c.r / 16;
    let g = 0x0f - c.g / 16;
    let b = 0x0f - c.b / 16;
    (0..=0x0a)
        .step_by(2)
        .map(|i| vec![0x07, 0x14, (i << 4) | g, (r << 4) | b, 0, 0, 0, 0])
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_color_sequence_is_byte_exact() {
        // OpenRGB InstantMouseController.cpp:65-80: RGB values are inverted
        // nibbles and are sent for DPI slots 0, 2, 4, 6, 8 and 10.
        let p = frames(Rgb {
            r: 0,
            g: 255,
            b: 128,
        });
        assert_eq!(
            p,
            vec![
                vec![0x07, 0x14, 0x00, 0xf7, 0, 0, 0, 0],
                vec![0x07, 0x14, 0x20, 0xf7, 0, 0, 0, 0],
                vec![0x07, 0x14, 0x40, 0xf7, 0, 0, 0, 0],
                vec![0x07, 0x14, 0x60, 0xf7, 0, 0, 0, 0],
                vec![0x07, 0x14, 0x80, 0xf7, 0, 0, 0, 0],
                vec![0x07, 0x14, 0xa0, 0xf7, 0, 0, 0, 0],
            ]
        );
    }
}
