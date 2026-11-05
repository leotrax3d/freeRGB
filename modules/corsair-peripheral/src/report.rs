// SPDX-License-Identifier: GPL-2.0-or-later
pub const REPORT_LEN: usize = 65;
pub fn lighting_control_header_bytes(device_type_byte: u8) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x05;
    p[3] = 0x02;
    p[5] = device_type_byte;
    p
}
pub fn mouse_direct(colors: &[freergb_core::Rgb]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x22;
    p[3] = colors.len().min(15) as u8;
    p[4] = 0;
    for (i, color) in colors.iter().enumerate().take(15) {
        p[i * 4 + 5] = i as u8;
        p[i * 4 + 6] = color.r;
        p[i * 4 + 7] = color.g;
        p[i * 4 + 8] = color.b;
    }
    p
}
pub fn mousemat_direct(colors: &[freergb_core::Rgb]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x22;
    p[3] = colors.len().min(15) as u8;
    p[4] = 0;
    for (i, color) in colors.iter().enumerate().take(15) {
        p[i * 3 + 5] = color.r;
        p[i * 3 + 6] = color.g;
        p[i * 3 + 7] = color.b;
    }
    p
}
pub fn stand_direct(colors: &[freergb_core::Rgb]) -> [u8; REPORT_LEN] {
    let remap = [0usize, 1, 2, 3, 5, 6, 7, 8, 4];
    let mut remapped = [[0u8; 3]; 9];
    for (i, color) in colors.iter().enumerate().take(9) {
        let slot = remap[i];
        remapped[slot][0] = color.r;
        remapped[slot][1] = color.g;
        remapped[slot][2] = color.b;
    }
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x22;
    p[3] = colors.len().min(9) as u8;
    p[4] = 0;
    for (i, rgb) in remapped.iter().enumerate().take(colors.len().min(9)) {
        p[i * 3 + 5] = rgb[0];
        p[i * 3 + 6] = rgb[1];
        p[i * 3 + 7] = rgb[2];
    }
    p
}
