// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: NvidiaESAController.cpp NvidiaESAController::SetZoneColor (Direct color)
pub const REPORT_LEN: usize = 4;
pub fn direct(zone: usize, c: Rgb) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = (0x42 + zone) as u8;
    // upstream: 0x0F - 0x0F * channel / 255.0, truncated to u8.
    p[1] = (0x0F as f32 - 0x0F as f32 * c.r as f32 / 255.0) as u8;
    p[2] = (0x0F as f32 - 0x0F as f32 * c.g as f32 / 255.0) as u8;
    p[3] = (0x0F as f32 - 0x0F as f32 * c.b as f32 / 255.0) as u8;
    p
}
