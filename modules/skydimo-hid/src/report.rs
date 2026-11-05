// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
const LED_COUNT: usize = 49;
const FRAME_SIZE: usize = LED_COUNT * 3;
const PACKET: usize = 64;
const DATA_SIZE: usize = 60;
const OFFSET: usize = 3;
fn crc8(data: &[u8], len: usize) -> u8 {
    let mut crc = 0u8;
    for &byte in &data[..len] {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
pub fn frames(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let mut frame_data = [0u8; FRAME_SIZE];
    for (i, c) in colors.iter().enumerate().take(LED_COUNT) {
        let off = i * 3;
        frame_data[off] = c.g;
        frame_data[off + 1] = c.r;
        frame_data[off + 2] = c.b;
    }
    let mut out = Vec::new();
    let mut frame_offset = 0usize;
    while frame_offset < FRAME_SIZE {
        let rem = FRAME_SIZE - frame_offset;
        let chunk_len = rem.min(DATA_SIZE);
        let start_led = (frame_offset / 3) as u8;
        let mut packet = [0u8; PACKET];
        packet[0] = 0x01; // report id
        packet[1] = start_led;
        packet[2] = 0x00;
        packet[OFFSET..OFFSET + chunk_len]
            .copy_from_slice(&frame_data[frame_offset..frame_offset + chunk_len]);
        packet[PACKET - 1] = crc8(&packet, PACKET - 1);
        out.push(packet.to_vec());
        frame_offset += DATA_SIZE;
    }
    // end packet
    let mut endp = [0u8; PACKET];
    endp[0] = 0x01;
    endp[1] = 0xFF;
    endp[2] = 0xFF;
    endp[3] = LED_COUNT as u8;
    endp[60] = 0x1E;
    // crc over all but last
    endp[PACKET - 1] = crc8(&endp, PACKET - 1);
    out.push(endp.to_vec());
    out
}
