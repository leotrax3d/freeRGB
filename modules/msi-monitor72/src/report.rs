// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream constants from `MSIMonitorController.h`.
pub const QUERY_PACKET_SIZE: usize = 64;
pub const QUERY_REPORT_ID: u8 = 0x01;
pub const QUERY_ACK: u8 = 0x5a;
pub const OPCODE_GET_VERSION: u8 = 0xb0;
pub const PACKET_SIZE: usize = 168;
pub const STORE_INDEX: usize = 167;
pub const ARRAY_A_INDEX: usize = 23;
pub const ARRAY_B_INDEX: usize = 95;
pub const ARRAY_SLOTS: usize = 24;
pub const MODE_STATIC: u8 = 1;
pub const SPEED_MEDIUM: u8 = 1;

/// Upstream: MSIMonitorController.cpp GetLayoutVersion. 64-byte query
/// `[0x01 0xB0]` on the short report channel.
pub fn version_query() -> Vec<u8> {
    let mut p = vec![0; QUERY_PACKET_SIZE];
    p[0] = QUERY_REPORT_ID;
    p[1] = OPCODE_GET_VERSION;
    p
}

/// Version from a query response; the reply must echo the 0x5A ACK at
/// byte 1 and carry the version at byte 3, exactly as upstream.
pub fn version_from_reply(r: &[u8]) -> Option<u8> {
    if r.len() > 3 && r[1] == QUERY_ACK {
        Some(r[3])
    } else {
        None
    }
}

/// Upstream: MSIMonitorControllerDetect.cpp `msi_monitor_72_layouts`
/// (version, LED count, right-to-left fill). The dual block versions
/// 11/13/15 and the advanced 0x82 versions 31/47 are disabled upstream and
/// are not listed; unknown versions keep the legacy 0x71 path, which the
/// module does not port.
pub const LAYOUTS: &[(u8, usize, bool)] = &[
    (34, 9, false),  /* MPG271QRXQD_OLED (MPG 322URX reports this) - verified */
    (5, 12, true),   /* MAG272R */
    (8, 24, false),  /* MAG301CR - Circle, plain line */
    (10, 9, false),  /* MAG273R */
    (12, 9, false),  /* MAG271VCR */
    (14, 14, false), /* NXG253R */
    (17, 8, true),   /* MAG2x5R */
    (20, 12, false), /* MAG301RF */
    (21, 18, false), /* MPG341QR */
    (22, 24, false), /* MPG321QRF */
    (24, 12, false), /* MAG321QR */
    (25, 24, false), /* MPG321UR */
    (28, 9, false),  /* MAG273CQR */
    (29, 24, true),  /* MAG401QR */
    (30, 21, true),  /* MAG325CQRF */
    (32, 7, false),  /* MAG274QRF */
    (33, 7, false),  /* MPG274URFQD */
    (36, 9, false),  /* MPG275CQRXF */
    (37, 22, false), /* MPG345CQRD */
    (38, 20, false), /* MAG325CQRFQDE2 */
    (39, 8, true),   /* MPG321CURX_QD_OLED */
    (40, 12, false), /* MAG322URDFE16 */
    (41, 14, true),  /* MPG242RX60N */
    (42, 22, false), /* MPG274URDFWE16 - Ctype, plain line */
    (43, 21, false), /* MPG346CQRFX24 */
    (44, 9, false),  /* MPG271QRQD_OLEDX50 */
    (46, 9, false),  /* MPG341CQRQD_OLEDX36 */
];

pub fn layout(version: u8) -> Option<(usize, bool)> {
    LAYOUTS
        .iter()
        .find(|(v, _, _)| *v == version)
        .map(|&(_, leds, rtl)| (leds, rtl))
}

/// Upstream: MSIMonitorController.cpp SetMode72 with the Direct path of
/// `RGBController_MSIMonitor72.cpp ApplyMode`: mode STATIC, medium speed,
/// brightness 100, no flash write (save = false). The engine paints from
/// the second inline colour array at index 95; the identical 11-byte
/// control blocks sit at index 1 and 12. RightToLeft layouts get the
/// per LED order mirrored so LED 0 stays at the left.
pub fn frame(colors: &[Rgb], reverse: bool) -> Vec<u8> {
    let mut colors: Vec<Rgb> = colors.to_vec();
    if reverse {
        colors.reverse();
    }
    let color1 = colors.first().copied().unwrap_or(Rgb { r: 0, g: 0, b: 0 });
    let mut data = vec![0u8; PACKET_SIZE];
    data[0] = 0x72;
    // Identical 11-byte control blocks at index 1 and 12
    for base in [1usize, 12] {
        data[base] = MODE_STATIC;
        data[base + 1] = color1.r;
        data[base + 2] = color1.g;
        data[base + 3] = color1.b;
        data[base + 4] = SPEED_MEDIUM;
        data[base + 5] = 100; // Direct brightness default
        data[base + 9] = 0x80; // user palette
        data[base + 10] = 0x00;
    }
    // Second inline colour array at index 95 (colour index above 15 would
    // route to the DragonShield region; the module packs into one array)
    for (slot, c) in colors.iter().enumerate().take(ARRAY_SLOTS) {
        let offset_b = ARRAY_B_INDEX + slot * 3;
        data[offset_b] = c.r;
        data[offset_b + 1] = c.g;
        data[offset_b + 2] = c.b;
    }
    // store-to-flash byte: never persist while streaming
    data[STORE_INDEX] = 0x00;
    data
}
