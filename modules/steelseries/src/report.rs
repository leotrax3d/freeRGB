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
        // Rival 3/5 use the separate mouse protocols (set_mouse_light_effect_all /
        // set_rival5_* builders)
        Proto::Rival3 | Proto::Rival5 => Vec::new(),
        // Siberia Direct sends only the SetColor command sequence
        Proto::Siberia350 => Vec::new(),
        // Arctis 5 Direct sends only per-zone SetColor sequences
        Proto::Arctis5 => Vec::new(),
        // Arctis Nova 3 Direct sends only per-LED SetDirect sequences
        Proto::ArctisNova3 => Vec::new(),
        // Aerox 3 Direct sends only per-zone SetColor (no effect packets;
        // SetLightEffectAll is a no-op upstream)
        Proto::Aerox3 => Vec::new(),
        // Aerox 5 same pattern as Aerox 3
        Proto::Aerox5 => Vec::new(),
        // upstream: SetLightEffectAll only reacts to effect 0x05, which is
        // never emitted by the Direct mode; Rival 3 Wireless ignores it
        Proto::AeroxWireless { .. } => Vec::new(),
        // QCK Direct sends no effect packets
        Proto::Qck => Vec::new(),
        // Apex 3 family Direct sends no separate effect packets
        Proto::Apex3 { .. } => Vec::new(),
        // upstream: RGBController_SteelSeriesSensei.cpp Direct mode sends no
        // effect packets, only per-zone SetColor
        Proto::Sensei => Vec::new(),
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
        // Rival 3/5 and Arctis 5 use their own protocol builders; routing
        // happens in lib.rs frame(), never through set_color()
        Proto::Rival3 | Proto::Rival5 => Vec::new(),
        // Arctis 5 (set_arctis5_color), Arctis Nova 3 (set_arctis_nova3_direct)
        // and Siberia (set_siberia_color) are handled in lib.rs frame()
        // Arctis 5 (set_arctis5_color), Arctis Nova 3 (set_arctis_nova3_direct)
        // and Siberia (set_siberia_color) are handled in lib.rs frame()
        Proto::Arctis5 => Vec::new(),
        Proto::ArctisNova3 => Vec::new(),
        // Aerox 3 uses aerox3_set_color(); handled in lib.rs frame()
        Proto::Aerox3 => Vec::new(),
        // Aerox 5 uses aerox5_set_color(); handled in lib.rs frame()
        Proto::Aerox5 => Vec::new(),
        // Aerox wireless rows use aerox_wireless_set_color(); handled in
        // lib.rs frame()
        Proto::AeroxWireless { .. } => Vec::new(),
        // QCK uses qck_set_colors(); handled in lib.rs frame()
        Proto::Qck => Vec::new(),
        // Apex 3 family uses the dedicated apex3 builders; handled in lib.rs
        // frame()
        Proto::Apex3 { .. } => Vec::new(),
        Proto::Siberia350 => Vec::new(),
        Proto::Sensei => vec![set_sensei_color(zone_id, c)],
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
/// upstream: SteelSeriesMouseController.h STEELSERIES_MOUSE_BRIGHTNESS_MAX
pub const MOUSE_BRIGHTNESS_MAX: u8 = 0x64;
/// upstream: SteelSeriesMouseController.h STEELSERIES_MOUSE_EFFECT_DIRECT
pub const MOUSE_DIRECT_VALUE: u8 = 0x04;
/// upstream: RGBController_SteelSeriesRival3.cpp DeviceUpdateMode() driven by
/// SteelSeriesRival3Controller.cpp SetLightEffectAll()
/// 4-byte write packet `[00 06 00 effect]` (report id 0x00 already inline).
pub fn set_mouse_light_effect_all(effect: u8) -> Vec<u8> {
    vec![0x00, 0x06, 0x00, effect]
}
/// upstream: SteelSeriesRival3Controller.cpp SetColor()
/// 8-byte write packet `[00 05 00 zone r g b brightness]`.
pub fn set_mouse_color(zone_id: u8, c: freergb_core::Rgb, brightness: u8) -> Vec<u8> {
    vec![0x00, 0x05, 0x00, zone_id, c.r, c.g, c.b, brightness]
}
/// upstream: SteelSeriesRival5Controller.cpp SetBrightness()
/// 64-byte feature report `[00 23 tier 00...]`; brightness quantized to the
/// tiers 0x64 / 0x32 / 0x19 / 0x0C / 0x00.
pub fn set_rival5_brightness(level: u8) -> Vec<u8> {
    let tier = if level >= 88 {
        0x64
    } else if level >= 63 {
        0x32
    } else if level >= 38 {
        0x19
    } else if level >= 13 {
        0x0C
    } else {
        0x00
    };
    let mut p = vec![0u8; 64];
    p[1] = 0x23;
    p[2] = tier;
    p
}
/// upstream: SteelSeriesRival5Controller.cpp UpdateLEDs()
/// 64-byte feature report `[00 21 FF 03 r g b ...]` with RGB for up to 10
/// zones starting at byte 4.
pub fn set_rival5_direct(colors: &[freergb_core::Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; 64];
    p[1] = 0x21;
    p[2] = 0xFF;
    p[3] = 0x03;
    for (i, c) in colors.iter().take(10).enumerate() {
        p[4 + (i * 3)] = c.r;
        p[5 + (i * 3)] = c.g;
        p[6 + (i * 3)] = c.b;
    }
    p
}
/// upstream: SteelSeriesSenseiController.cpp SetColor()
/// 65-byte packet `[5B 00 zone ... 13=01 1B=01 rgb rgb]` plus 0x00 report id
/// prefix (66 bytes total).
pub fn set_sensei_color(zone_id: u8, c: freergb_core::Rgb) -> Vec<u8> {
    write(66, |b| {
        b[0x00] = 0x5B;
        b[0x02] = zone_id;
        b[0x13] = 0x01; // Static color flag
        b[0x1B] = 0x01; // Number of colors
        b[0x1C] = c.r;
        b[0x1D] = c.g;
        b[0x1E] = c.b;
        b[0x1F] = c.r;
        b[0x20] = c.g;
        b[0x21] = c.b;
    })
}
/// upstream: SteelSeriesSenseiController.cpp Save()
/// 9-byte packet with only the leading 0x59 plus 0x00 report id.
pub fn sensei_save() -> Vec<u8> {
    write(10, |b| b[0] = 0x59)
}
/// upstream: SteelSeriesSiberiaController.cpp send_usb_msg()
/// Wraps a command in the 16-byte Siberia framing:
/// `[01 report][00 magic][cmd][len][payload ...]`.
fn siberia_packet(cmd: u8, payload: &[u8]) -> Vec<u8> {
    let mut p = vec![0u8; 16];
    p[0] = 0x01;
    p[1] = 0x00;
    p[2] = cmd;
    p[3] = payload.len() as u8;
    p[4..4 + payload.len()].copy_from_slice(payload);
    p
}
/// upstream: SteelSeriesSiberiaController.cpp SetColor()
/// Four 16-byte write packets: two magic commands, the color payload, and
/// an apply command.
pub fn set_siberia_color(c: freergb_core::Rgb) -> Vec<Vec<u8>> {
    vec![
        siberia_packet(0x95, &[0x80, 0xbf]),    // Command 1
        siberia_packet(0x80, &[0x52, 0x20]),    // Command 2
        siberia_packet(0x83, &[c.r, c.g, c.b]), // Command 3 (Set color)
        siberia_packet(0x93, &[0x03, 0x80]),    // Command 4
    ]
}
/// upstream: SteelSeriesArctis5Controller.cpp SetColor()
/// Five 37-byte write packets (report id 0x06): two pre-change packets, the
/// color packet, and two apply-to-zone/apply packets.
pub fn set_arctis5_color(zone_id: u8, c: freergb_core::Rgb) -> Vec<Vec<u8>> {
    const SIZE: usize = 37;
    let mut pre1 = vec![0u8; SIZE];
    pre1[0x00] = 0x06;
    pre1[0x01] = 0x81;
    pre1[0x02] = 0x43;
    pre1[0x03] = 0x01;
    pre1[0x04] = 0x22;
    let mut pre2 = vec![0u8; SIZE];
    pre2[0x00] = 0x06;
    pre2[0x01] = 0x81;
    pre2[0x02] = 0x43;
    pre2[0x03] = 0x01;
    pre2[0x04] = 0x23;
    let mut color = vec![0u8; SIZE];
    color[0x00] = 0x06;
    color[0x01] = 0x8A;
    color[0x02] = 0x42;
    color[0x04] = 0x20;
    color[0x05] = 0x41;
    color[0x07] = c.r;
    color[0x08] = c.g;
    color[0x09] = c.b;
    color[0x0A] = 0xFF;
    color[0x0B] = 0x32;
    color[0x0C] = 0xC8;
    color[0x0D] = 0xC8;
    let mut apply_zone = vec![0u8; SIZE];
    apply_zone[0x00] = 0x06;
    apply_zone[0x01] = 0x8A;
    apply_zone[0x02] = 0x42;
    apply_zone[0x04] = 0x20;
    apply_zone[0x05] = 0x41;
    apply_zone[0x06] = 0x08;
    apply_zone[0x07] = zone_id;
    apply_zone[0x08] = 0x01;
    let mut apply_to = vec![0u8; SIZE];
    apply_to[0x00] = 0x06;
    apply_to[0x01] = 0x8A;
    apply_to[0x02] = 0x42;
    apply_to[0x04] = 0x20;
    apply_to[0x05] = 0x60;
    apply_to[0x06] = zone_id;
    let mut apply = vec![0u8; SIZE];
    apply[0x00] = 0x06;
    apply[0x01] = 0x8A;
    apply[0x02] = 0x42;
    apply[0x04] = 0x20;
    apply[0x05] = 0x05;
    vec![pre1, pre2, color, apply_zone, apply_to, apply]
}
/// upstream: SteelSeriesArctisNova3Controller.h ARCTIS_NOVA3_REPORT_SIZE /
/// ARCTIS_NOVA3_APPLY_REPORT_SIZE / ARCTIS_NOVA3_REPORT_ID
const ARCTIS_NOVA3_REPORT_SIZE: usize = 521;
const ARCTIS_NOVA3_APPLY_REPORT_SIZE: usize = 64;
const ARCTIS_NOVA3_REPORT_ID: u8 = 0x06;
/// upstream: SteelSeriesArctisNova3Controller.cpp SelectZone()
/// 64-byte select/enable packet `[06 A5 (01/02) state]` (zone 0 uses 0x02).
fn arctis_nova3_select(zone_id: u8, state: u8) -> Vec<u8> {
    let mut p = vec![0u8; ARCTIS_NOVA3_APPLY_REPORT_SIZE];
    p[0] = ARCTIS_NOVA3_REPORT_ID;
    p[1] = 0xA5;
    p[2] = if zone_id == 0 { 0x02 } else { 0x01 };
    p[3] = state;
    p
}
/// upstream: SteelSeriesArctisNova3Controller.cpp ApplyZone()
/// 64-byte apply packet: zone 0 -> [06 09], zone 1 -> [06 A3].
fn arctis_nova3_apply(zone_id: u8) -> Vec<u8> {
    let mut p = vec![0u8; ARCTIS_NOVA3_APPLY_REPORT_SIZE];
    p[0] = ARCTIS_NOVA3_REPORT_ID;
    p[1] = if zone_id == 0 { 0x09 } else { 0xA3 };
    p
}
/// upstream: SteelSeriesArctisNova3Controller.cpp SetDirect()
/// 521-byte effect packet plus select (0x0A) and apply packets.
pub fn set_arctis_nova3_direct(zone_id: u8, c: freergb_core::Rgb) -> Vec<Vec<u8>> {
    let mut p = vec![0u8; ARCTIS_NOVA3_REPORT_SIZE];
    p[0] = ARCTIS_NOVA3_REPORT_ID;
    p[1] = 0xAA;
    p[2] = if zone_id == 0 { 0x01 } else { 0x00 };
    p[0x0D] = 0x98;
    p[0x0E] = 0x3A;
    p[0x15] = 0x55;
    p[0x17] = 0x96;
    p[0x1D] = 0x01;
    p[0x25] = 0x02;
    p[0x2D] = c.r;
    p[0x2E] = c.g;
    p[0x2F] = c.b;
    p[0x30] = 0xFF;
    vec![
        p,
        arctis_nova3_select(zone_id, 0x0A),
        arctis_nova3_apply(zone_id),
    ]
}
/// upstream: SteelSeriesAerox3Controller.h STEELSERIES_AEORX3_PACKET_SIZE
const AEROX3_PACKET_SIZE: usize = 65;
/// upstream: SteelSeriesAerox3Controller.cpp SendInit()
/// 65-byte feature report `[00 2D 00...]` sent once at construction time.
pub fn aerox3_init() -> Vec<u8> {
    let mut p = vec![0u8; AEROX3_PACKET_SIZE];
    p[1] = 0x2D;
    p
}
/// upstream: SteelSeriesAerox3Controller.cpp SetColor()
/// 65-byte write `[00 21 (1 << zone) rgb at 3 + zone*3]`.
pub fn aerox3_set_color(zone_id: u8, c: freergb_core::Rgb) -> Vec<u8> {
    let mut p = vec![0u8; AEROX3_PACKET_SIZE];
    p[1] = 0x21;
    p[2] = 1u8 << zone_id;
    let offset = 3 + zone_id as usize * 3;
    p[offset] = c.r;
    p[offset + 1] = c.g;
    p[offset + 2] = c.b;
    p
}
/// upstream: SteelSeriesAerox3Controller.cpp SetBrightness()
/// 65-byte write `[00 23 brightness]`.
pub fn aerox3_set_brightness(brightness: u8) -> Vec<u8> {
    let mut p = vec![0u8; AEROX3_PACKET_SIZE];
    p[1] = 0x23;
    p[2] = brightness;
    p
}
/// upstream: SteelSeriesAerox3Controller.cpp Save()
/// 65-byte write `[00 11 00]` (not used by the module API; tested builder).
pub fn aerox3_save() -> Vec<u8> {
    let mut p = vec![0u8; AEROX3_PACKET_SIZE];
    p[1] = 0x11;
    p
}
/// upstream: SteelSeriesAerox5Controller.cpp SetColor()
/// 65-byte write. Zones 0..2 use cmd 0x21 with 1<<zone at byte 2 and RGB at
/// 3+zone*3; zone 3 ("Reactive") uses cmd 0x26 with RGB at 0x04. Upstream
/// sleeps 2 ms between packets; the module protocol has no sleeps, so calls
/// are emitted back to back (noted in PROVENANCE).
pub fn aerox5_set_color(zone_id: u8, c: freergb_core::Rgb) -> Vec<u8> {
    let mut p = vec![0u8; AEROX3_PACKET_SIZE];
    let (cmd, zone_byte, offset): (u8, u8, usize) = match zone_id {
        0 => (0x21, 0x01, 0x03),
        1 => (0x21, 0x02, 0x06),
        2 => (0x21, 0x04, 0x09),
        3 => (0x26, 0x01, 0x04),
        _ => return p,
    };
    p[1] = cmd;
    p[2] = zone_byte;
    p[offset] = c.r;
    p[offset + 1] = c.g;
    p[offset + 2] = c.b;
    p
}
/// upstream: SteelSeriesAerox5Controller.cpp SetBrightness()
/// Upstream writes a 3-byte initialiser to hid_write with the 65-byte packet
/// size; the intended packet is `[00 23 brightness]` zero-padded to 65 bytes.
pub fn aerox5_set_brightness(brightness: u8) -> Vec<u8> {
    let mut p = vec![0u8; AEROX3_PACKET_SIZE];
    p[1] = 0x23;
    p[2] = brightness;
    p
}
/// upstream: SteelSeriesAerox5Controller.cpp Save() - same `[00 11 00]`
/// packet intent as Aerox 3.
pub fn aerox5_save() -> Vec<u8> {
    let mut p = vec![0u8; AEROX3_PACKET_SIZE];
    p[1] = 0x11;
    p
}
/// upstream: SteelSeriesAeroxWirelessController.h
/// STEELSERIES_AEROX_WIRELESS_PACKET_SIZE / STEELSERIES_AEROX_WIRELESS_FLAG
const AEROX_WIRELESS_PACKET_SIZE: usize = 64;
const AEROX_WIRELESS_FLAG: u8 = 0b01000000;
/// upstream: SteelSeriesAeroxWirelessController.h aerox_3_wireless table
/// (also used by every Aerox 5 wireless/wired row)
pub const AEROX_WIRELESS_A3_LEDS: [(&str, u8); 3] = [("Front", 0), ("Middle", 1), ("Rear", 2)];
/// upstream: SteelSeriesAeroxWirelessController.h aerox_9 table
pub const AEROX_WIRELESS_A9_LEDS: [(&str, u8); 4] =
    [("Front", 0), ("Middle", 1), ("Rear", 2), ("Reactive", 3)];
/// upstream: SteelSeriesAeroxWirelessController.h rival_3_wireless table
pub const AEROX_WIRELESS_RIVAL3_LEDS: [(&str, u8); 1] = [("Scroll Wheel", 1)];
/// upstream: SteelSeriesAeroxWirelessController.cpp SendInit()
/// 64-byte feature report `[00 2D 00...]`.
pub fn aerox_wireless_init() -> Vec<u8> {
    let mut p = vec![0u8; AEROX_WIRELESS_PACKET_SIZE];
    p[1] = 0x2D;
    p
}
/// upstream: SteelSeriesAeroxWirelessController.cpp SetColor()
/// 64-byte write: direct rows `[00 21 01 zone rgb@4..7]`; Aerox 9 zone 3
/// uses cmd 0x26 with byte 3 = 0; Rival 3 Wireless uses its own
/// `[00 13 zone rgb@3..6]` packet. The 0x40 flag is ORed into byte 1 for
/// wireless rows.
pub fn aerox_wireless_set_color(
    wireless: bool,
    aerox9: bool,
    rival3: bool,
    zone_id: u8,
    c: freergb_core::Rgb,
) -> Vec<u8> {
    let mut p = vec![0u8; AEROX_WIRELESS_PACKET_SIZE];
    if rival3 {
        // upstream: SetColor case RIVAL_3_WIRELESS
        p[1] = 0x13;
        p[2] = zone_id;
        p[3] = c.r;
        p[4] = c.g;
        p[5] = c.b;
        return p;
    }
    p[1] = 0x21;
    p[2] = 0x01;
    if zone_id == 3 && aerox9 {
        // upstream: 9 Wireless "Reactive" zone uses the 0x26 command
        p[3] = 0x00;
        p[1] = 0x26;
    } else {
        p[3] = zone_id;
    }
    p[4] = c.r;
    p[5] = c.g;
    p[6] = c.b;
    if wireless {
        p[1] |= AEROX_WIRELESS_FLAG;
    }
    p
}
/// upstream: SteelSeriesAeroxWirelessController.cpp SetBrightness()
/// `[00 23 level]` with the 0x40 flag for wireless rows.
pub fn aerox_wireless_set_brightness(wireless: bool, level: u8) -> Vec<u8> {
    let mut p = vec![0u8; AEROX_WIRELESS_PACKET_SIZE];
    p[1] = 0x23;
    p[2] = level;
    if wireless {
        p[1] |= AEROX_WIRELESS_FLAG;
    }
    p
}
/// upstream: SteelSeriesQCKMatController.cpp SetColors()
/// 525-byte feature report with both LED colors plus a 65-byte apply write.
pub fn qck_set_colors(c0: freergb_core::Rgb, c1: freergb_core::Rgb) -> Vec<Vec<u8>> {
    let mut buf = vec![0u8; 525];
    buf[0x00] = 0x00;
    buf[0x01] = 0x0E;
    buf[0x03] = 0x02;
    buf[0x08] = 0xFF;
    buf[0x09] = 0x32;
    buf[0x0A] = 0xC8;
    buf[0x0E] = 0x01;
    buf[0x14] = 0xFF;
    buf[0x15] = 0x32;
    buf[0x16] = 0xC8;
    buf[0x19] = 0x01;
    buf[0x1A] = 0x01;
    buf[0x1C] = 0x01;
    buf[0x05] = c0.r;
    buf[0x06] = c0.g;
    buf[0x07] = c0.b;
    buf[0x11] = c1.r;
    buf[0x12] = c1.g;
    buf[0x13] = c1.b;
    let mut apply = vec![0u8; 65];
    apply[1] = 0x0D;
    vec![buf, apply]
}
/// upstream: SteelSeriesApexTZoneController.cpp SetColor()
/// Two 33-byte writes: brightness `[00 0A 00 brightness]` then colors
/// `[00 0B 00 r g b ...]` for 10 LEDs starting at byte 3.
pub fn apex3_tzone_set_color(colors: &[freergb_core::Rgb], brightness: u8) -> Vec<Vec<u8>> {
    let mut set_brightness = vec![0u8; 33];
    set_brightness[1] = 0x0A;
    set_brightness[3] = brightness;
    let mut set_colors = vec![0u8; 33];
    set_colors[1] = 0x0B;
    for (i, c) in colors.iter().take(10).enumerate() {
        set_colors[i * 3 + 3] = c.r;
        set_colors[i * 3 + 4] = c.g;
        set_colors[i * 3 + 5] = c.b;
    }
    vec![set_brightness, set_colors]
}
/// upstream: SteelSeriesApexTZoneController.cpp Save()
/// Two 33-byte writes `[00 06 00 08]` and `[00 09 00 00]` (not used by the
/// module API; tested builder).
pub fn apex3_tzone_save() -> Vec<Vec<u8>> {
    let mut a = vec![0u8; 33];
    a[1] = 0x06;
    a[3] = 0x08;
    let mut b = vec![0u8; 33];
    b[1] = 0x09;
    vec![a, b]
}
/// upstream: SteelSeriesApex8ZoneController.cpp SetColor()
/// 65-byte write `[00 (0x21 + mode) FF rgb@3..]` for 8 LEDs. Only the Direct
/// mode (mode byte 0 -> 0x21) is exposed.
pub fn apex3_8zone_set_color(colors: &[freergb_core::Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; 65];
    p[1] = 0x21;
    p[2] = 0xFF;
    for (i, c) in colors.iter().take(8).enumerate() {
        p[i * 3 + 3] = c.r;
        p[i * 3 + 4] = c.g;
        p[i * 3 + 5] = c.b;
    }
    p
}
/// upstream: SteelSeriesApex8ZoneController.cpp SetBrightness()
/// 65-byte write `[00 23 brightness]`.
pub fn apex3_8zone_set_brightness(brightness: u8) -> Vec<u8> {
    let mut p = vec![0u8; 65];
    p[1] = 0x23;
    p[2] = brightness;
    p
}
// upstream: RGBController_SteelSeriesApex3.cpp SetupZones() led names
pub const APEX3_LED_NAMES: [&str; 10] = [
    "LED 0", "LED 1", "LED 2", "LED 3", "LED 4", "LED 5", "LED 6", "LED 7", "LED 8", "LED 9",
];
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
