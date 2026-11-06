// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream `RoccatKoneAimoController`: persistent 46-byte colour report.
pub const REPORT_SIZE: usize = 46;

/// Number of logical LEDs (per `RGBController_RoccatKoneAimo`: 2 + 4 + 4
/// plus two single LEDs = 11).
pub const LED_COUNT: usize = 11;
/// LED index within the 11-colour layout for each upstream zone. Three
/// zones carry multiple colours; the channel indices they map to in the
/// upstream 46-byte buffer are 0 (wheel), 1..=4 (left strip), 5..=8
/// (right strip), 9 (lower left), 10 (lower right).
pub const ZONE_LED_OFFSETS: [(usize, usize); 5] = [(0, 1), (1, 4), (1, 4), (9, 1), (10, 1)];

/// Upstream `SendInit`: 6-byte feature report `[0E 06 01 01 00 FF]`.
pub fn init() -> Vec<u8> {
    vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]
}

/// Upstream `SendChannelColors` + `SendUpdate`: persistent 46-byte buffer
/// `0D 2E` then, for each LED `n`, the R/G/B at `2 + n * 4` (the fourth
/// byte is left untouched and is usually 0). All colours are painted, then a
/// 3-byte `get_feature` follow-up occurs (the caller handles that).
pub fn set_colors(colors: &[Rgb]) -> Vec<u8> {
    let mut buf = vec![0u8; REPORT_SIZE];
    buf[0] = 0x0d;
    buf[1] = 0x2e;
    for (i, c) in colors.iter().enumerate().take(LED_COUNT) {
        let b = 2 + i * 4;
        buf[b] = c.r;
        buf[b + 1] = c.g;
        buf[b + 2] = c.b;
    }
    buf
}
