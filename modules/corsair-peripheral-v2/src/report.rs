// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET: usize = 65;
pub fn direct(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[0] = 8;
    p[1] = 3;
    p[2] = 1;
    p[3] = 1;
    for (n, c) in colors.iter().take(20).enumerate() {
        let q = 4 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn effect(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[0] = 8;
    p[1] = 3;
    p[2] = 0x22;
    p[3] = 3;
    for (n, c) in colors.iter().take(16).enumerate() {
        let q = 5 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}

pub fn set_mode(write_cmd: u8, mode: u8) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[1] = write_cmd;
    p[2] = 3;
    p[3] = 3;
    p[5] = mode;
    p
}

pub fn lighting_control(write_cmd: u8) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[1] = write_cmd;
    p[2] = 1;
    p[3] = 0x5f;
    p
}

pub fn block_packets(write_cmd: u8, data: &[u8]) -> Vec<Vec<u8>> {
    let mut first = vec![0; 1025];
    first[1] = write_cmd;
    first[2] = 0x30;
    first[4] = (data.len() & 0xff) as u8;
    first[5] = (data.len() >> 8) as u8;
    let first_len = data.len().min(1017);
    first[8..8 + first_len].copy_from_slice(&data[..first_len]);
    let mut packets = vec![first];
    let mut offset = first_len;
    while offset < data.len() {
        let mut next = vec![0; 1025];
        next[1] = write_cmd;
        next[2] = 0x31;
        let n = (data.len() - offset).min(1021);
        next[4..4 + n].copy_from_slice(&data[offset..offset + n]);
        packets.push(next);
        offset += n;
    }
    let mut stop = vec![0; 65];
    stop[1] = write_cmd;
    stop[2] = 0x32;
    stop[3] = 1;
    packets.push(stop);
    packets
}
