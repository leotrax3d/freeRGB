// SPDX-License-Identifier: GPL-2.0-or-later
//! Packet builders for the MSI Mystic Light 112-byte motherboard controller.
//! Every byte below mirrors MSIMotherboardCommon.h /
//! MSIMotherboard112Controller.cpp (OpenRGB commit
//! bbc77470a7cba56af60989e54ba910d5ac66a3ac).
/// upstream: MSIMotherboardCommon.h FeaturePacket_112 (report id + 11 zones + save)
pub const PACKET_112: usize = 112;
/// upstream: MSIMotherboardControllerDetect.cpp probe buffer size
pub const PROBE_LEN: usize = 200;
/// upstream: MSIMotherboard112Controller.cpp constructor sets
/// numof_onboard_leds = 7
pub const ONBOARD_ZONE_LEDS: usize = 7;
/// upstream: MSIMotherboard112Controller.cc uses 64-byte read/write packets
pub const FW_PACKET_LEN: usize = 64;
/// upstream: MSIMotherboardCommon.h ZoneData is 10 bytes per zone
pub const ZONE_SIZE: usize = 10;
/// ZoneData offsets inside FeaturePacket_112 (byte 0 is the report id 0x52)
/// upstream: MSIMotherboardCommon.h FeaturePacket_112 field order
pub const ZONE_OFFSETS_112: [usize; 11] = [1, 11, 21, 31, 41, 51, 61, 71, 81, 91, 101];
// Zone field offsets (relative to the zone base):
//   +0 effect, +1..3 color, +4 speedAndBrightnessFlags, +5..7 color2,
//   +8 colorFlags, +9 padding
// Flag macros: MSIMotherboardCommon.h
//   MSI_BRIGHTNESS_LEVEL_100 = 10, brightness shift 2  -> 0x28
//   MSI_COLOR_FLAG_USE_CUSTOM_COLOR = 1 << 7           -> 0x80
const BRIGHTNESS_100_FLAGS: u8 = 10 << 2; // MSI_BRIGHTNESS_LEVEL_100 << SHIFT
pub const USE_CUSTOM_COLOR: u8 = 1 << 7;
pub const MSI_MODE_STATIC: u8 = 1;
/// upstream: MSIMotherboardCommon.h MSI_COLOR_FLAG_SYNC_SETTING_ONBOARD
pub const SYNC_ONBOARD: u8 = 1 << 0;
/// upstream: MSIMotherboard112Controller.cpp SetLedColor() + GetZoneData()
/// LED value (MSI_ZONE) -> zone slot index in the 112-byte packet. Upstream
/// adds +1 to every onboard zone value before the lookup, so the master
/// zone 13 lands in on_board_led_1 and value 19 is dropped (nullptr).
pub fn zone_slot_for_led(value: u8) -> Option<usize> {
    match value {
        // MSI_ZONE_J_RGB_1 -> j_rgb_1
        1 => Some(0),
        // MSI_ZONE_J_RAINBOW_1 -> j_rainbow_1
        8 => Some(1),
        // MSI_ZONE_J_CORSAIR -> j_corsair_1
        11 => Some(2),
        // MSI_ZONE_ON_BOARD_LED_0..6 with the +1 remap
        // 13->14 on_board_led_1, ..., 18->19 on_board_led_6
        13..=18 => Some(4 + (value as usize - 13) + 1),
        _ => None,
    }
}
/// Writes a direct-mode ZoneData block the way
/// SSIMotherboard112Controller prepares `zone_based_per_led_data`:
/// effect = MSI_MODE_STATIC, color/color2 = LED color,
/// speedAndBrightnessFlags = 100% brightness, colorFlags = custom color.
fn write_zone_direct(p: &mut [u8], slot: usize, c: freergb_core::Rgb) {
    let off = ZONE_OFFSETS_112[slot];
    p[off] = MSI_MODE_STATIC;
    p[off + 1..off + 4].copy_from_slice(&[c.r, c.g, c.b]);
    p[off + 4] = BRIGHTNESS_100_FLAGS;
    p[off + 5..off + 8].copy_from_slice(&[c.r, c.g, c.b]);
    p[off + 8] = USE_CUSTOM_COLOR;
    // padding byte stays 0
}
/// upstream: MSIMotherboard112Controller.cpp Update() in direct mode
/// Builds the 112-byte zone_based_per_led_data packet:
/// [0x52, zones..., save_data=0]. Zone colors follow the upstream LED
/// table (JRGB1, JRAINBOW1, JCORSAIR, 7 onboard LEDs); the onboard
/// master zone keeps its constructor defaults (color black) and value 19
/// is dropped, mirroring SetLedColor's remap.
pub fn direct_packet_112(colors: &[freergb_core::Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; PACKET_112];
    p[0] = 0x52;
    // Zones with upstream defaults: effect STATIC, flags zero except the
    // initialized ones below.
    for slot in 0..11 {
        p[ZONE_OFFSETS_112[slot]] = MSI_MODE_STATIC;
    }
    // JRGB1 / JRainbow1 / JCorsair: constructor sets brightness 100 and
    // custom color; frame colors fill both color fields.
    for idx in 0..3usize {
        let c = colors.get(idx).copied().unwrap_or_default();
        write_zone_direct(p.as_mut_slice(), idx, c);
    }
    // Onboard LEDs: master zone (slot 4) keeps color 0 but has the direct
    // flags from the constructor; LEDs 1..6 receive colors 3..9 of the
    // frame (values 13..18); value 19 is dropped upstream.
    let master = freergb_core::Rgb::default();
    write_zone_direct(p.as_mut_slice(), 4, master);
    for k in 0..6usize {
        let c = colors.get(3 + k).copied().unwrap_or_default();
        write_zone_direct(p.as_mut_slice(), 5 + k, c);
    }
    p[111] = 0; // save_data
    p
}
/// upstream: MSIMotherboard112Controller.cpp ReadFwVersion() - APROM request
pub fn fw_request(report_id: u8) -> Vec<u8> {
    let mut p = vec![0u8; FW_PACKET_LEN];
    p[0] = 0x01;
    p[1] = report_id;
    for b in p.iter_mut().skip(2) {
        *b = 0xCC;
    }
    p
}
/// Firmware version string from an APROM/LDROM response.
/// upstream: ReadFwVersion() - high nibble . low nibble of byte 2
pub fn fw_string(response: &[u8]) -> String {
    let high = response[2] >> 4;
    let low = response[2] & 0x0F;
    format!("{}.{}", high, low)
}
/// upstream: MSIMotherboardCommon.h FeaturePacket_162
pub const PACKET_162: usize = 162;
/// ZoneData offsets inside FeaturePacket_162:
/// j_rgb_1, j_rainbow_1, j_corsair_1, j_corsair_outer, on_board, led_1..10,
/// j_rgb_2 (see MSIMotherboardCommon.h)
pub const ZONE_OFFSETS_162: [usize; 16] = [
    1, 11, 21, 31, 41, 51, 61, 71, 81, 91, 101, 111, 121, 131, 141, 151,
];
/// upstream: MSIMotherboard162Controller.cpp GetZoneData()
/// MSI_ZONE value -> zone slot index in the 162-byte packet.
pub fn zone_slot_162(value: u8) -> Option<usize> {
    match value {
        1 => Some(0),  // j_rgb_1
        2 => Some(15), // j_rgb_2
        8 => Some(1),  // j_rainbow_1
        9 => Some(14), // upstream maps J_RAINBOW_2 to on_board_led_10
        13 => Some(4), // on_board_led
        14..=23 => Some(4 + (value as usize - 13)),
        11 => Some(2), // j_corsair_1
        _ => None,
    }
}
/// upstream: MSIMotherboard162Controller.cpp board_configs table
/// (PID, onboard LED count, supported zone values in RGBController order).
pub fn board_config_162(pid: u16) -> (usize, &'static [u8]) {
    match pid {
        0x1720 | 0x7B12 | 0x7B17 | 0x7B85 | 0x7B92 => (10, &[1, 2, 8, 11]),
        0x7B18 => (6, &[1, 2, 8]),
        0x7B50 => (6, &[1, 2, 8, 9]),
        0xB926 => (3, &[8]),
        _ => (10, &[1, 2, 8, 11]),
    }
}
/// Flags/macros mirrored from MSIMotherboardCommon.h (shared with 162).
const SYNC_ONBOARD_FLAG: u8 = 1 << 0;
const SYNC_JRGB_FLAG: u8 = 1 << 7;
/// upstream: MSIMotherboard162Controller.cpp SetMode() with
/// mode = MSI_MODE_STATIC (the "Direct" mode value), no speed, brightness 100.
/// The base part (zone <= 13) writes effect/flags and clears the sync bits
/// (mode <= MSI_MODE_DOUBLE_FLASHING); the second part (zone >= 13) writes
/// zone+1 for the multi-LED onboard chain.
pub fn set_mode_162(data: &mut [u8], value: u8) {
    let static_effect = MSI_MODE_STATIC;
    if (value as u16) <= 13 {
        if let Some(slot) = zone_slot_162(value) {
            let off = ZONE_OFFSETS_162[slot];
            data[off] = static_effect;
            data[off + 4] = ((data[off + 4] & !SYNC_JRGB_FLAG) & 0xFC) | BRIGHTNESS_100_FLAGS;
            data[off + 8] |= USE_CUSTOM_COLOR;
            data[off + 8] &= !SYNC_ONBOARD_FLAG;
            data[off + 9] = 0;
        }
    }
    if value >= 13 {
        if let Some(slot) = zone_slot_162(value + 1) {
            let off = ZONE_OFFSETS_162[slot];
            data[off] = static_effect;
            data[off + 4] = ((data[off + 4] & !SYNC_JRGB_FLAG) & 0xFC) | BRIGHTNESS_100_FLAGS;
            data[off + 8] |= USE_CUSTOM_COLOR;
            data[off + 9] = 0;
        }
    }
}
/// upstream: MSIMotherboard162Controller.cpp SetZoneColor() with
/// color2 == color1 (RGBController_UpdateLed passes rgb twice).
pub fn set_zone_color_162(data: &mut [u8], value: u8, c: freergb_core::Rgb) {
    if (value as u16) <= 13 {
        if let Some(slot) = zone_slot_162(value) {
            let off = ZONE_OFFSETS_162[slot];
            data[off + 1..off + 4].copy_from_slice(&[c.r, c.g, c.b]);
            data[off + 5..off + 8].copy_from_slice(&[c.r, c.g, c.b]);
        }
    }
    if value >= 13 {
        if let Some(slot) = zone_slot_162(value + 1) {
            let off = ZONE_OFFSETS_162[slot];
            data[off + 1..off + 4].copy_from_slice(&[c.r, c.g, c.b]);
            data[off + 5..off + 8].copy_from_slice(&[c.r, c.g, c.b]);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    const A: Rgb = Rgb { r: 1, g: 2, b: 3 };
    const B: Rgb = Rgb { r: 4, g: 5, b: 6 };
    const C: Rgb = Rgb { r: 7, g: 8, b: 9 };
    // upstream: MSIMotherboard112Controller.cpp SetLedColor + Update(direct)
    #[test]
    fn direct_packet_112_is_byte_exact() {
        let colors = [A, B, C, A, B, C, A, B, C, A];
        let p = direct_packet_112(&colors);
        assert_eq!(p.len(), 112);
        assert_eq!(p[0], 0x52);
        // j_rgb_1 zone (offset 1): effect 1, rgb, 0x28, rgb2, 0x80, pad 0
        assert_eq!(&p[1..11], &[1, 1, 2, 3, 0x28, 1, 2, 3, 0x80, 0]);
        // j_rainbow_1 (11) and j_corsair_1 (21)
        assert_eq!(&p[11..21], &[1, 4, 5, 6, 0x28, 4, 5, 6, 0x80, 0]);
        assert_eq!(&p[21..31], &[1, 7, 8, 9, 0x28, 7, 8, 9, 0x80, 0]);
        // j_corsair_outerll120 (31): never written in direct mode, only the
        // default STATIC effect byte
        assert_eq!(&p[31..41], &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        // on_board_led master (41): flags set by the constructor, color black
        assert_eq!(&p[41..51], &[1, 0, 0, 0, 0x28, 0, 0, 0, 0x80, 0]);
        // on_board_led_1..6 receive frame colors 3..9 (values 13..18)
        assert_eq!(&p[51..61], &[1, 1, 2, 3, 0x28, 1, 2, 3, 0x80, 0]);
        assert_eq!(&p[61..71], &[1, 4, 5, 6, 0x28, 4, 5, 6, 0x80, 0]);
        assert_eq!(&p[71..81], &[1, 7, 8, 9, 0x28, 7, 8, 9, 0x80, 0]);
        assert_eq!(&p[81..91], &[1, 1, 2, 3, 0x28, 1, 2, 3, 0x80, 0]);
        assert_eq!(&p[91..101], &[1, 4, 5, 6, 0x28, 4, 5, 6, 0x80, 0]);
        assert_eq!(&p[101..111], &[1, 7, 8, 9, 0x28, 7, 8, 9, 0x80, 0]);
        assert_eq!(p[111], 0);
    }
    // upstream: MSIMotherboard112Controller.cpp SetLedColor value remap
    #[test]
    fn zone_slot_mapping_matches_upstream_remap() {
        assert_eq!(zone_slot_for_led(1), Some(0));
        assert_eq!(zone_slot_for_led(8), Some(1));
        assert_eq!(zone_slot_for_led(11), Some(2));
        assert_eq!(zone_slot_for_led(13), Some(5));
        assert_eq!(zone_slot_for_led(18), Some(10));
        assert_eq!(zone_slot_for_led(19), None);
        assert_eq!(zone_slot_for_led(0), None);
    }
    // upstream: MSIMotherboard112Controller.cpp ReadFwVersion()
    #[test]
    fn fw_requests_are_byte_exact() {
        let p = fw_request(0xB0);
        assert_eq!(&p[0..3], &[0x01, 0xB0, 0xCC]);
        assert!(p.iter().skip(2).all(|b| *b == 0xCC));
        let q = fw_request(0xB6);
        assert_eq!(&q[0..2], &[0x01, 0xB6]);
        assert_eq!(fw_string(&[0, 0, 0x25, 0]), "2.5");
        assert_eq!(fw_string(&[0, 0, 0x11, 0]), "1.1");
    }
}
