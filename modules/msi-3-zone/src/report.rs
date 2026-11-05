// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: MSI3ZoneController.cpp SetLEDs (always seven 8-byte feature reports)
pub const REPORT_LEN: usize = 8;
pub fn frames(colors: &[Rgb]) -> Vec<[u8; REPORT_LEN]> {
    let c = |i: usize| colors.get(i).copied().unwrap_or(colors[0]);
    let last = c(3);
    let mut out = Vec::with_capacity(7);
    for zone in 1u8..=4u8 {
        let color = match zone {
            1 => c(0),
            2 => c(1),
            3 => c(2),
            _ => c(3),
        };
        out.push([1, 2, 64, zone, color.r, color.g, color.b, 236]);
    }
    for zone in 5u8..=7u8 {
        out.push([1, 2, 64, zone, last.r, last.g, last.b, 236]);
    }
    out
}