// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: RGBController_NZXTHue2.cpp / NZXTHue2Controller.cpp: at most two
/// 20-LED groups per channel are sent (first direct + optional second + apply),
/// so a channel drives at most 40 LEDs like upstream does.
pub const LED_PER_GROUP: usize = 20;
pub fn device_list_request() -> Vec<u8> {
    let mut p = vec![0; 64];
    p[0] = 0x20;
    p[1] = 3;
    p
}
pub fn direct(channel: usize, group: usize, colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 64];
    p[0] = 0x22;
    p[1] = 0x10 | (group as u8);
    p[2] = 1u8 << (channel as u8);
    for (i, c) in colors.iter().take(LED_PER_GROUP).enumerate() {
        let q = 4 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    p
}
pub fn apply(channel: usize) -> Vec<u8> {
    let mut p = vec![0; 64];
    p[0] = 0x22;
    p[1] = 0xa0;
    p[2] = 1u8 << (channel as u8);
    p[4] = 1;
    p[7] = 0x28;
    p[10] = 0x80;
    p[12] = 0x32;
    p[15] = 1;
    p
}
/// upstream: NZXTHue2Controller.cpp ReadDeviceList switch on device id.
/// Returns 0 for unknown ids like upstream's default case.
pub fn parse_device_list(reply: &[u8], channels: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for chan in 0..channels {
        let start = 0x0f + 6 * chan;
        let mut zone = Vec::new();
        for id in reply.get(start..start + 6).unwrap_or(&[]) {
            zone.push(match id {
                1 | 4 | 10 => 10,
                2 | 5 | 11 | 12 | 23 | 24 => 8,
                6 => 6,
                8 => 14,
                9 => 15,
                13 | 14 => 18,
                15 => 20,
                16 => 8, // Kraken X3 ring
                17 => 1, // Kraken X3 logo
                19..=20 => 18,
                21 | 22 => 20,
                28 => 16,
                29..=31 => 24,
                _ => 0,
            });
        }
        out.push(zone);
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_is_byte_exact() {
        let p = direct(2, 0, &[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(&p[..8], &[0x22, 0x10, 4, 0, 2, 1, 3, 0]);
    }
    #[test]
    fn apply_is_byte_exact() {
        assert_eq!(
            &apply(1)[..16],
            &[0x22, 0xa0, 2, 0, 1, 0, 0, 0x28, 0, 0, 0x80, 0, 0x32, 0, 0, 1]
        );
    }
    #[test]
    fn device_list_sizes_are_decoded() {
        let mut p = vec![0; 64];
        p[0x0f] = 4;
        p[0x10] = 8;
        assert_eq!(parse_device_list(&p, 1)[0], vec![10, 14, 0, 0, 0, 0]);
    }
    #[test]
    fn device_list_ids_match_upstream_switch() {
        // upstream: NZXTHue2Controller.cpp ReadDeviceList case table
        let mut p = vec![0; 64];
        p[0x0f] = 0x10; // Kraken X3 ring -> 8
        p[0x10] = 0x11; // Kraken X3 logo -> 1
        p[0x11] = 0x13; // F120 RGB -> 18
        p[0x12] = 0x14; // F140 RGB -> 18
        p[0x13] = 0x15; // F120 RGB Duo -> 20
        p[0x14] = 0x16; // F140 RGB Duo -> 20
        p[0x15] = 0x1E; // Kraken Elite ring -> 24
        assert_eq!(parse_device_list(&p, 1)[0], vec![8, 1, 18, 18, 20, 20]);
        // 6 devices per channel: id 0x15 lands in channel 1 slot 0
        let mut q = vec![0; 64];
        q[0x0f + 6] = 0x16;
        assert_eq!(parse_device_list(&q, 2)[0], vec![0, 0, 0, 0, 0, 0]);
        assert_eq!(parse_device_list(&q, 2)[1], vec![20, 0, 0, 0, 0, 0]);
    }
}
