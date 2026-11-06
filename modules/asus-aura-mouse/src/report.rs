// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream: AsusAuraMouseController.h / AsusAuraMouseDevices.h.
pub const PACKET_SIZE: usize = 65;

/// Zone ids from AsusAuraMouseDevices.h (only ones used by Direct devices).
pub const ZONE_LOGO: u8 = 0;
pub const ZONE_SCROLL: u8 = 1;
pub const ZONE_UNDERGLOW: u8 = 2;

/// Zone id plus the colour slots it paints into the direct colour buffer.
pub type ZoneSlots = (u8, &'static [usize]);

/// Per-PID lighting layout from `aura_mouse_led_maps`:
/// (pid, [(zone id, colour slots to paint within the direct colour buffer)]).
/// Each openRGB "LED" is one zone; the zone's colour is painted into every
/// one of its slots in the 5-per-packet direct buffer.
pub const LED_MAPS: &[(u16, u8, &[ZoneSlots])] = &[
    (0x1a70, 1, &[(ZONE_LOGO, &[0])]), // Gladius III AimPoint USB
    (0x1a72, 1, &[(ZONE_LOGO, &[0])]), // Gladius III AimPoint 2.4
    (0x1a66, 1, &[(ZONE_LOGO, &[0])]), // Keris AimPoint USB
    (0x1a68, 1, &[(ZONE_LOGO, &[0])]), // Keris AimPoint 2.4
    (
        0x1a18,
        9,
        &[
            (ZONE_LOGO, &[7]),
            (ZONE_SCROLL, &[8]),
            (ZONE_UNDERGLOW, &[0, 1, 2, 3, 4, 5, 6]),
        ],
    ),
    (
        0x1a1a,
        9,
        &[
            (ZONE_LOGO, &[7]),
            (ZONE_SCROLL, &[8]),
            (ZONE_UNDERGLOW, &[0, 1, 2, 3, 4, 5, 6]),
        ],
    ),
    (
        0x1977,
        5,
        &[
            (ZONE_LOGO, &[0]),
            (ZONE_SCROLL, &[1]),
            (ZONE_UNDERGLOW, &[2, 3, 4]),
        ],
    ),
    (
        0x1979,
        5,
        &[
            (ZONE_LOGO, &[0]),
            (ZONE_SCROLL, &[1]),
            (ZONE_UNDERGLOW, &[2, 3, 4]),
        ],
    ),
    (0x1a88, 2, &[(ZONE_LOGO, &[0]), (ZONE_SCROLL, &[1])]), // Strix Impact III
];

pub fn led_map(pid: u16) -> (u8, &'static [ZoneSlots]) {
    LED_MAPS
        .iter()
        .find(|(p, _, _)| *p == pid)
        .map(|&(_, amount, slots)| (amount, slots))
        .unwrap_or((0, &[]))
}

/// Upstream: AuraMouseController.cpp SendDirect. Each LED (one per zone)
/// paints its slot(s) in the colour buffer; 5 colors per 65-byte packet
/// `00 51 29 <count> 00 <offset> R G B ...` (one write per packet).
pub fn direct(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    let mut led = 0usize;
    while led < colors.len() {
        let count = (colors.len() - led).min(5);
        let mut p = vec![0u8; PACKET_SIZE];
        p[1] = 0x51;
        p[2] = 0x29;
        p[3] = count as u8;
        p[5] = led as u8;
        for (i, c) in colors[led..led + count].iter().enumerate() {
            p[6 + i * 3] = c.r;
            p[7 + i * 3] = c.g;
            p[8 + i * 3] = c.b;
        }
        packets.push(p);
        led += 5;
    }
    packets
}
