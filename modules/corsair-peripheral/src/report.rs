// SPDX-License-Identifier: GPL-2.0-or-later
pub const REPORT_LEN: usize = 65;
pub fn lighting_control_header_bytes(device_type_byte: u8) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x05;
    p[3] = 0x02;
    p[5] = device_type_byte;
    p
}
pub fn mouse_direct(colors: &[freergb_core::Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x22;
    p[3] = colors.len().min(15) as u8;
    p[4] = 0;
    for (i, c) in colors.iter().enumerate().take(15) {
        p[i * 4 + 5] = i as u8;
        p[i * 4 + 6] = c.r;
        p[i * 4 + 7] = c.g;
        p[i * 4 + 8] = c.b;
    }
    p
}
pub fn mousemat_direct(colors: &[freergb_core::Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x22;
    p[3] = colors.len().min(15) as u8;
    p[4] = 0;
    for (i, c) in colors.iter().enumerate().take(15) {
        p[i * 3 + 5] = c.r;
        p[i * 3 + 6] = c.g;
        p[i * 3 + 7] = c.b;
    }
    p
}
pub fn stand_direct(colors: &[freergb_core::Rgb]) -> Vec<u8> {
    let remap = [0usize, 1, 2, 3, 5, 6, 7, 8, 4];
    let mut remapped = [[0u8; 3]; 9];
    for (i, c) in colors.iter().enumerate().take(9) {
        let s = remap[i];
        remapped[s][0] = c.r;
        remapped[s][1] = c.g;
        remapped[s][2] = c.b;
    }
    let mut p = vec![0u8; REPORT_LEN];
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
fn stream_packet(packet_id: u8, sz: u8, data: &[u8]) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x7F;
    p[2] = packet_id;
    p[3] = sz;
    p[4] = 0;
    if sz as usize <= data.len() && 5 + sz as usize <= p.len() {
        p[5..5 + sz as usize].copy_from_slice(&data[..sz as usize]);
    }
    p
}

fn submit(channel: u8, packet_count: u8, finish: u8) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_LEN];
    p[0] = 0;
    p[1] = 0x07;
    p[2] = 0x28;
    p[3] = channel;
    p[4] = packet_count;
    p[5] = finish;
    p
}
pub fn keyboard_normal(colors: &[freergb_core::Rgb]) -> Vec<Vec<u8>> {
    let mut r = [0u8; 168];
    let mut g = [0u8; 168];
    let mut b = [0u8; 168];
    for i in 0..colors.len().min(113) {
        r[i] = colors[i].r;
        g[i] = colors[i].g;
        b[i] = colors[i].b;
    }
    let out = vec![
        // red
        stream_packet(1, 60, &r[..60]),
        stream_packet(2, 60, &r[60..120]),
        stream_packet(3, 24, &r[120..144]),
        submit(1, 3, 1),
        // green
        stream_packet(1, 60, &g[..60]),
        stream_packet(2, 60, &g[60..120]),
        stream_packet(3, 24, &g[120..144]),
        submit(2, 3, 1),
        // blue
        stream_packet(1, 60, &b[..60]),
        stream_packet(2, 60, &b[60..120]),
        stream_packet(3, 24, &b[120..144]),
        submit(3, 3, 2),
    ];
    out
}
