// SPDX-License-Identifier: GPL-2.0-or-later
/// upstream: XPGSummonerController.cpp SendInitialize/SendColorDataPacket
pub const REPORT_LEN: usize = 265;
pub fn init_report() -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x07;
    p[1] = 0xEA;
    p[2] = 0x00;
    p[3] = 0x00;
    p
}
pub fn chunk(packet_id: usize, data: &[u8]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x07;
    p[1] = 0xA3;
    p[2] = 0x08;
    p[3] = 0x00;
    p[4] = packet_id as u8;
    p[5] = 0x00;
    let n = data.len().min(256);
    for i in 0..n {
        p[6 + i] = data[i];
    }
    p
}
