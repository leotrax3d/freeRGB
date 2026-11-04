// SPDX-License-Identifier: GPL-2.0-or-later
//! Packet builders for the SteelSeries Rival sub-controller family.
//! Every byte below is taken from upstream
//! SteelSeriesRivalController.cpp / RGBController_SteelSeriesRival.cpp
//! (OpenRGB commit bbc77470a7cba56af60989e54ba910d5ac66a3ac).
use crate::table::Proto;
/// upstream: SteelSeriesRivalController.h STEELSERIES_RIVAL_EFFECT_DIRECT
pub const EFFECT_DIRECT: u8 = 0x01;
/// upstream: SteelSeriesRivalController.cpp send_usb_msg()
/// Prepends a 0x00 report id to a raw packet and returns the buffer to write.
pub fn write(prefixed_len: usize, fill: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let mut p = vec![0u8; prefixed_len];
    fill(&mut p[1..]);
    p[0] = 0x00;
    p
}
/// upstream: SteelSeriesRivalController.cpp SetLightEffect()
/// 9-byte packet `[cmd, zone, effect, 0...]` plus 0x00 report id (10 bytes).
/// Rival 700 returns early (no packet); Rival 600/650 fall into the C++
/// switch default, leaving cmd/zone at 0x00 - reproduced byte-exactly.
pub fn set_light_effect(proto: Proto, zone_id: u8, effect: u8) -> Vec<u8> {
    if proto == Proto::Rival700 {
        // upstream: SteelSeriesRivalController.cpp SetLightEffect case RIVAL_700: return
        return Vec::new();
    }
    write(10, |b| {
        match proto {
            Proto::Rival100 => b[0] = 0x07,
            Proto::Rival300 => {
                b[0] = 0x07;
                b[1] = zone_id + 1;
            }
            // upstream: switch default - Rival 600/650 leave cmd/zone at 0x00
            _ => {}
        }
        b[2] = effect;
    })
}
/// upstream: SteelSeriesRivalController.cpp SaveMode()
/// 9-byte packet with only the leading 0x09 plus 0x00 report id.
pub fn save_mode() -> Vec<u8> {
    write(10, |b| b[0] = 0x09)
}
/// upstream: SteelSeriesRivalController.cpp SetLightEffectAll()
pub fn set_light_effect_all(proto: Proto, effect: u8) -> Vec<Vec<u8>> {
    match proto {
        Proto::Rival100 => vec![set_light_effect(proto, 0, effect)],
        Proto::Rival300 => vec![
            set_light_effect(proto, 0, effect),
            set_light_effect(proto, 1, effect),
        ],
        Proto::Rival650 => (0x10..0x18)
            .map(|z| set_light_effect(proto, z, effect))
            .collect(),
        Proto::Rival600 | Proto::Rival700 => Vec::new(),
    }
}
/// upstream: SteelSeriesRivalController.cpp SetColor()
/// Direct-mode single-LED color update. Returns the packets to write (and,
/// for Rival 700, a packet to send as a feature report).
pub fn set_color(proto: Proto, zone_id: u8, c: freergb_core::Rgb) -> Vec<Vec<u8>> {
    match proto {
        Proto::Rival650 => set_rival650_color(zone_id, c),
        Proto::Rival600 => set_rival600_color(zone_id, c),
        Proto::Rival700 => vec![set_rival700_color(zone_id, c)],
        Proto::Rival100 | Proto::Rival300 => {
            let cmd: u8;
            let zone: u8;
            match proto {
                Proto::Rival100 => {
                    cmd = 0x05;
                    zone = 0x00;
                }
                _ => {
                    cmd = 0x08;
                    zone = zone_id + 1;
                }
            }
            vec![write(10, |b| {
                b[0] = cmd;
                b[1] = zone;
                b[2] = c.r;
                b[3] = c.g;
                b[4] = c.b;
            })]
        }
    }
}
/// upstream: SteelSeriesRivalController.cpp SetRival600Color()
/// 7-byte write packet; byte 3 is a 1 << zone_id bit mask. The C++ code
/// passes the raw 7 bytes to hid_write (report id 0x1c is byte 0).
pub fn set_rival600_color(zone_id: u8, c: freergb_core::Rgb) -> Vec<Vec<u8>> {
    vec![vec![0x1c, 0x27, 0x00, 1u8 << zone_id, c.r, c.g, c.b]]
}
/// upstream: SteelSeriesRivalController.cpp SetRival650Color()
/// Four 60-byte packets (each with 0x00 report id prefix, 61 bytes total).
pub fn set_rival650_color(zone_id: u8, c: freergb_core::Rgb) -> Vec<Vec<u8>> {
    let p1 = write(61, |b| {
        b[0x00] = 0x03;
        b[0x04] = 0x30;
        b[0x06] = 0x10;
        b[0x07] = 0x27;
        b[0x16] = 0x01;
        b[0x1E] = 0x04;
        b[0x1F] = c.r;
        b[0x20] = c.g;
        b[0x21] = c.b;
        b[0x22] = 0xFF;
        b[0x27] = 0xFF;
        b[0x29] = 0x54;
        b[0x2C] = 0xFF;
        b[0x2D] = 0x54;
        b[0x2E] = c.r;
        b[0x2F] = c.g;
        b[0x30] = c.b;
        b[0x31] = 0x56;
    });
    let p2 = write(61, |b| {
        b[0x00] = 0x03;
        b[0x02] = 0x30;
        b[0x04] = 0x2C;
    });
    let p3 = write(61, |b| {
        b[0x00] = 0x05;
        b[0x02] = zone_id;
        b[0x03] = 0xFF;
        b[0x08] = 0x5C;
    });
    let p4 = write(61, |b| {
        b[0x00] = 0x1C;
        b[0x02] = 0x55;
        b[0x04] = 0x46;
    });
    vec![p1, p2, p3, p4]
}
/// upstream: SteelSeriesRivalController.cpp SetRival700Color()
/// 578-byte buffer sent as a feature report with a 0x00 report id prefix
/// (579 bytes total).
pub fn set_rival700_color(zone_id: u8, c: freergb_core::Rgb) -> Vec<u8> {
    write(579, |b| {
        b[0x00] = 0x05;
        b[0x02] = zone_id;
        b[0x03] = c.r;
        b[0x04] = c.g;
        b[0x05] = c.b;
        b[0x0b] = zone_id;
        b[0x0c] = 0x01;
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Proto;
    const RGB1: freergb_core::Rgb = freergb_core::Rgb {
        r: 0x11,
        g: 0x22,
        b: 0x33,
    };
    // upstream: SteelSeriesRivalController.cpp SetLightEffect case RIVAL_100
    #[test]
    fn set_light_effect_rival100_is_byte_exact() {
        assert_eq!(
            set_light_effect(Proto::Rival100, 0, EFFECT_DIRECT),
            vec![0x00, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]
        );
    }
    // upstream: SteelSeriesRivalController.cpp SetLightEffect case RIVAL_300
    #[test]
    fn set_light_effect_rival300_is_byte_exact() {
        assert_eq!(
            set_light_effect(Proto::Rival300, 1, EFFECT_DIRECT),
            vec![0x00, 0x07, 0x02, 0x01, 0, 0, 0, 0, 0, 0]
        );
    }
    // upstream: SteelSeriesRivalController.cpp SetLightEffect default arm
    // (Rival 650 falls through, cmd/zone stay zero)
    #[test]
    fn set_light_effect_rival650_falls_through_to_zeros() {
        assert_eq!(
            set_light_effect(Proto::Rival650, 0x12, EFFECT_DIRECT),
            vec![0x00, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]
        );
    }
    // upstream: SteelSeriesRivalController.cpp SetLightEffect case RIVAL_700
    #[test]
    fn set_light_effect_rival700_sends_nothing() {
        assert!(set_light_effect(Proto::Rival700, 0, EFFECT_DIRECT).is_empty());
        assert!(set_light_effect_all(Proto::Rival700, EFFECT_DIRECT).is_empty());
    }
    // upstream: SteelSeriesRivalController.cpp SetLightEffectAll
    #[test]
    fn set_light_effect_all_matches_upstream_zone_sets() {
        assert_eq!(
            set_light_effect_all(Proto::Rival300, EFFECT_DIRECT).len(),
            2
        );
        assert_eq!(
            set_light_effect_all(Proto::Rival650, EFFECT_DIRECT).len(),
            8
        );
        assert_eq!(
            set_light_effect_all(Proto::Rival600, EFFECT_DIRECT).len(),
            0
        );
        assert_eq!(
            set_light_effect_all(Proto::Rival100, EFFECT_DIRECT).len(),
            1
        );
    }
    // upstream: SteelSeriesRivalController.cpp SetColor case RIVAL_100
    #[test]
    fn set_color_rival100_is_byte_exact() {
        assert_eq!(
            set_color(Proto::Rival100, 0, RGB1),
            vec![vec![0x00, 0x05, 0x00, 0x11, 0x22, 0x33, 0, 0, 0, 0]]
        );
    }
    // upstream: SteelSeriesRivalController.cpp SetColor case RIVAL_300
    // (zone_id + 1, so the two LEDs become zone 1 and 2)
    #[test]
    fn set_color_rival300_is_byte_exact() {
        assert_eq!(
            set_color(Proto::Rival300, 0, RGB1),
            vec![vec![0x00, 0x08, 0x01, 0x11, 0x22, 0x33, 0, 0, 0, 0]]
        );
        assert_eq!(
            set_color(Proto::Rival300, 1, RGB1),
            vec![vec![0x00, 0x08, 0x02, 0x11, 0x22, 0x33, 0, 0, 0, 0]]
        );
    }
    // upstream: SteelSeriesRivalController.cpp SetRival600Color
    // (1 << zone_id mask, RGB at bytes 4..7, no report-id prefix)
    #[test]
    fn set_rival600_color_is_byte_exact() {
        assert_eq!(
            set_rival600_color(0x02, RGB1),
            vec![vec![0x1c, 0x27, 0x00, 0x04, 0x11, 0x22, 0x33]]
        );
        assert_eq!(set_rival600_color(0x00, RGB1)[0][3], 0x01);
    }
    // upstream: SteelSeriesRivalController.cpp SetRival650Color
    #[test]
    fn set_rival650_color_is_byte_exact() {
        let p = set_rival650_color(0x12, RGB1);
        assert_eq!(p.len(), 4);
        assert_eq!(p.iter().map(|q| q.len()).collect::<Vec<_>>(), [61; 4]);
        // Packet 1: header magic and both RGB triples.
        assert_eq!(
            &p[0][0x01..0x09],
            &[0x03, 0x00, 0x00, 0x00, 0x30, 0x00, 0x10, 0x27]
        );
        assert_eq!(p[0][0x17], 0x01); // usb_buf[0x16]
        assert_eq!(
            &p[0][0x20..0x23],
            &[0x11, 0x22, 0x33] // usb_buf[0x1F..0x21]
        );
        assert_eq!(p[0][0x23], 0xFF);
        assert_eq!(p[0][0x28], 0xFF);
        assert_eq!(p[0][0x2A], 0x54);
        assert_eq!(p[0][0x2D], 0xFF);
        assert_eq!(p[0][0x2E], 0x54);
        assert_eq!(&p[0][0x2F..0x32], &[0x11, 0x22, 0x33]);
        assert_eq!(p[0][0x32], 0x56);
        // Packet 2: commit magic.
        assert_eq!(&p[1][1..6], &[0x03, 0x00, 0x30, 0x00, 0x2C]);
        // Packet 3: mousekey zone, 0xFF flag, 0x5C tail.
        assert_eq!(&p[2][1..10], &[0x05, 0x00, 0x12, 0xFF, 0, 0, 0, 0, 0x5C]);
        // Packet 4: 0x1C / 0x55 / 0x46 trailer.
        assert_eq!(&p[3][1..6], &[0x1C, 0x00, 0x55, 0x00, 0x46]);
        // All other bytes zero.
        let zeroed = |q: &Vec<u8>, keep: &[usize]| {
            q.iter()
                .enumerate()
                .filter(|(i, _)| !keep.contains(i))
                .all(|(_, b)| *b == 0)
        };
        assert!(zeroed(&p[1], &[0, 1, 3, 5]));
        assert!(zeroed(&p[3], &[0, 1, 3, 5]));
    }
    // upstream: SteelSeriesRivalController.cpp SetRival700Color
    #[test]
    fn set_rival700_color_is_byte_exact() {
        let p = set_rival700_color(1, RGB1);
        assert_eq!(p.len(), 579);
        assert_eq!(p[0], 0x00);
        assert_eq!(p[1], 0x05);
        assert_eq!(p[3], 0x01);
        assert_eq!(&p[4..7], &[0x11, 0x22, 0x33]);
        assert_eq!(p[0x0C], 0x01);
        assert_eq!(p[0x0D], 0x01);
        assert!(p[7..=0x0B].iter().all(|b| *b == 0));
        assert!(p[0x0E..].iter().all(|b| *b == 0));
    }
    // upstream: SteelSeriesRivalController.cpp SaveMode
    #[test]
    fn save_mode_is_byte_exact() {
        assert_eq!(save_mode(), vec![0x00, 0x09, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
}
