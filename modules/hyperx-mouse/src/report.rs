// SPDX-License-Identifier: GPL-2.0-or-later
//! Packet builders for the HyperX Pulsefire Surge mouse.
//! Every byte mirrors HyperXPulsefireSurgeController.cpp (OpenRGB commit
//! bbc77470a7cba56af60989e54ba910d5ac66a3ac).
/// upstream: HyperXPulsefireSurgeController.cpp HID packet size (264 bytes)
pub const PACKET_LEN: usize = 264;
/// Number of RGB plane LEDs + the single logo LED.
pub const SURGE_LEDS: usize = 33;
/// upstream: HyperXPulsefireSurgeController.cpp SendDirect()
/// 264-byte feature report `[07 14 00 A0 ...]` with the color planes for the
/// 32-LED strip split at 0x08 (red), 0x28 (green), 0x48 (blue) and the
/// singular logo LED at 0x6C.
pub fn surge_direct(colors: &[freergb_core::Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; PACKET_LEN];
    p[0x00] = 0x07;
    p[0x01] = 0x14;
    p[0x02] = 0x00;
    p[0x03] = 0xA0;
    for (i, c) in colors.iter().take(32).enumerate() {
        p[0x08 + i] = c.r;
        p[0x28 + i] = c.g;
        p[0x48 + i] = c.b;
    }
    let logo = colors.get(32).copied().unwrap_or_default();
    p[0x6C] = logo.r;
    p[0x6D] = logo.g;
    p[0x6E] = logo.b;
    p
}
/// upstream: HyperXPulsefireFPSProController.cpp SendDirect()
/// 264-byte feature report `[07 0A r g b ...0xA0@0x08]` for the single logo
/// LED (FPS Pro and Pulsefire Core share the path).
pub fn fps_direct(c: freergb_core::Rgb) -> Vec<u8> {
    let mut p = vec![0u8; PACKET_LEN];
    p[0x00] = 0x07;
    p[0x01] = 0x0A;
    p[0x02] = c.r;
    p[0x03] = c.g;
    p[0x04] = c.b;
    p[0x08] = 0xA0;
    p
}
/// upstream: HyperXPulsefireRaidController.cpp SendColors()
/// 264-byte feature report `[07 0A rgb rgb 0xA0@8]` for the two RAID LEDs
/// (Scroll Wheel, Logo).
pub fn raid_direct(c0: freergb_core::Rgb, c1: freergb_core::Rgb) -> Vec<u8> {
    let mut p = vec![0u8; PACKET_LEN];
    p[0] = 0x07;
    p[1] = 0x0A;
    p[2] = c0.r;
    p[3] = c0.g;
    p[4] = c0.b;
    p[5] = c1.r;
    p[6] = c1.g;
    p[7] = c1.b;
    p[8] = 0xA0;
    p
}
pub fn haste_setup() -> Vec<u8> {
    let mut p = vec![0u8; 65];
    p[1] = 0x04;
    p[2] = 0xF2;
    p[8] = 0x02;
    p
}
pub fn haste_color(c: freergb_core::Rgb) -> Vec<u8> {
    let mut p = vec![0u8; 65];
    p[1] = 0x81;
    p[2] = c.r;
    p[3] = c.g;
    p[4] = c.b;
    p[8] = 0x02;
    p
}
/// upstream: HyperXPulsefireDartController.cpp SendDirect (Direct/static mode)
/// 65-byte HID write: `[00 D2 led 00 08 R G B R G B 0x64 00 ...]`,
/// mode = static (0x00), duplicate RGB, brightness = 0x64 (max), speed = 0x00.
pub fn dart_direct(c: freergb_core::Rgb, led: u8) -> Vec<u8> {
    let mut p = vec![0u8; 65];
    p[0] = 0x00;
    p[1] = 0xD2;
    p[2] = led;
    p[3] = 0x00; // static
    p[4] = 0x08;
    p[5] = c.r;
    p[6] = c.g;
    p[7] = c.b;
    p[8] = c.r;
    p[9] = c.g;
    p[10] = c.b;
    p[11] = 0x64; // max brightness
    p[12] = 0x00; // speed (static)
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    const A: Rgb = Rgb { r: 1, g: 2, b: 3 };
    const L: Rgb = Rgb { r: 4, g: 5, b: 6 };
    // upstream: HyperXPulsefireSurgeController.cpp SendDirect
    #[test]
    fn surge_direct_is_byte_exact() {
        let mut colors = vec![A; 32];
        colors.push(L);
        let p = surge_direct(&colors);
        assert_eq!(p.len(), 264);
        assert_eq!(&p[0..4], &[0x07, 0x14, 0x00, 0xA0]);
        assert_eq!(p[0x08], 1);
        assert_eq!(p[0x27], 1);
        assert_eq!(p[0x28], 2);
        assert_eq!(p[0x47], 2);
        assert_eq!(p[0x48], 3);
        assert_eq!(p[0x67], 3);
        assert_eq!(&p[0x6C..0x6F], &[4, 5, 6]);
        assert!(p[0x68..0x6C].iter().all(|b| *b == 0));
        assert!(p[0x6F..].iter().all(|b| *b == 0));
        assert!(p[0x04..0x08].iter().all(|b| *b == 0));
    }
    // upstream: HyperXPulsefireFPSProController.cpp SendDirect
    #[test]
    fn fps_direct_is_byte_exact() {
        let p = fps_direct(A);
        assert_eq!(p.len(), 264);
        assert_eq!(&p[0..9], &[0x07, 0x0A, 1, 2, 3, 0, 0, 0, 0xA0]);
        assert!(p[9..].iter().all(|b| *b == 0));
    }
    // upstream: HyperXPulsefireRaidController.cpp SendColors
    #[test]
    fn raid_direct_is_byte_exact() {
        let p = raid_direct(A, L);
        assert_eq!(p.len(), 264);
        assert_eq!(&p[0..9], &[0x07, 0x0A, 1, 2, 3, 4, 5, 6, 0xA0]);
        assert!(p[9..].iter().all(|b| *b == 0));
    }
    // upstream: HyperXPulsefireDartController.cpp SendDirect
    #[test]
    fn dart_direct_is_byte_exact() {
        let p = dart_direct(A, 0x10);
        assert_eq!(p.len(), 65);
        assert_eq!(
            &p[0..13],
            &[0x00, 0xD2, 0x10, 0x00, 0x08, 1, 2, 3, 1, 2, 3, 0x64, 0x00]
        );
        assert!(p[13..].iter().all(|b| *b == 0));
    }
}
