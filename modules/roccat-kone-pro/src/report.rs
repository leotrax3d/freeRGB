// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream constants from `RoccatKoneProController.h`.
pub const DIRECT_REPORT_ID: u8 = 0x0d;
pub const DIRECT_MODE_BYTE: u8 = 0x0b;
pub const CONTROL_REPORT_ID: u8 = 0x0e;

/// Upstream: RoccatKoneProController::SetupDirectMode (=SwitchControl(true)):
/// 6-byte feature report `[0E 06 01 01 00 FF]`.
pub fn enter_direct() -> Vec<u8> {
    vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]
}

/// Upstream: RoccatKoneProController.cpp SendDirect: 11-byte feature report
/// `[0D 0B R G B R G B]` (two LEDs, no padding).
pub fn direct(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; 11];
    p[0] = DIRECT_REPORT_ID;
    p[1] = DIRECT_MODE_BYTE;
    for (i, c) in colors.iter().enumerate().take(2) {
        let b = 2 + i * 3;
        p[b] = c.r;
        p[b + 1] = c.g;
        p[b + 2] = c.b;
    }
    p
}
