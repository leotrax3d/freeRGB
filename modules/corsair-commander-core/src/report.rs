// SPDX-License-Identifier: GPL-2.0-or-later
//! Packet builders for the Corsair Commander Core controller.
//! Every byte mirrors CorsairCommanderCoreController.cpp (OpenRGB commit
//! bbc77470a7cba56af60989e54ba910d5ac66a3ac).
// upstream: CorsairCommanderCoreController.h packet size macros
pub const PACKET_V1: usize = 1025;
pub const PACKET_V2: usize = 97;
pub const PACKET_V3: usize = 65;
pub const PACKET_XT: usize = 385;
// upstream: CorsairCommanderCoreController.h RGB data buffer macros
pub const RGB_DATA_LENGTH: usize = 699;
pub const RGB_DATA_LENGTH_XT: usize = 1224;
pub const NUM_CHANNELS: usize = 6;
/// PIDs from CorsairCommanderCoreController.h
pub const PID_CORE: u16 = 0x0C1C;
pub const PID_CORE2: u16 = 0x0C32;
pub const PID_CORE_XT: u16 = 0x0C2A;
/// upstream: SendCommand() - command frame `[00 08 cmd0 cmd1 data...]`
/// zero-padded to the negotiated packet size.
pub fn command_packet(packet_size: usize, cmd: [u8; 2], data: Option<&[u8]>) -> Vec<u8> {
    let mut buf = vec![0u8; packet_size];
    buf[0] = 0x00;
    buf[1] = 0x08;
    buf[2] = cmd[0];
    buf[3] = cmd[1];
    if let Some(d) = data {
        buf[4..4 + d.len()].copy_from_slice(d);
    }
    buf
}
/// upstream: WriteData() first packet: `[len_lo, len_hi, 0, 0, type0, type1,
/// data...]` where real_len = data_len + 2 (the 2 data-type bytes are counted).
pub fn data_packet(data_type: [u8; 2], chunk: &[u8], total_len: usize) -> Vec<u8> {
    let real_len = (total_len + 2) as u16;
    let mut buf = vec![0u8; chunk.len() + 6];
    buf[0] = (real_len & 0xFF) as u8;
    buf[1] = (real_len >> 8) as u8;
    buf[2] = 0x00;
    buf[3] = 0x00;
    buf[4] = data_type[0];
    buf[5] = data_type[1];
    buf[6..].copy_from_slice(chunk);
    buf
}
/// upstream: SetDirectColor() - packs the zone colors into the direct-mode
/// RGB buffer: 3 bytes per LED; after every zone except the first, 3 bytes
/// per unused LED up to the 34-LED channel maximum are padded.
/// Returns the used prefix length as well (packet_offset).
pub fn rgb_buffer(
    xt: bool,
    zone_counts: &[usize],
    colors: &[freergb_core::Rgb],
) -> (Vec<u8>, usize) {
    let packet_len = if xt {
        RGB_DATA_LENGTH_XT
    } else {
        RGB_DATA_LENGTH
    };
    let mut buf = vec![0u8; packet_len];
    let mut offset = 0usize;
    let mut led_idx = 0usize;
    for (zone_idx, &count) in zone_counts.iter().enumerate() {
        for i in led_idx..led_idx + count {
            let c = colors.get(i).copied().unwrap_or_default();
            buf[offset] = c.r;
            buf[offset + 1] = c.g;
            buf[offset + 2] = c.b;
            offset += 3;
        }
        led_idx += count;
        if zone_idx != 0 {
            offset += 3 * (34usize.saturating_sub(count));
        }
    }
    (buf, offset)
}
/// upstream: InitController()/ReadData()/WriteData() endpoint constants
pub mod endpoints {
    /// version / keepalive query
    pub const VERSION: [u8; 2] = [0x02, 0x13];
    /// wake/hardware mode commands
    pub const WAKE_HW: [u8; 2] = [0x01, 0x03];
    pub const WAKE_DATA_INIT: [u8; 2] = [0x00, 0x02];
    pub const HW_MODE_DATA: [u8; 2] = [0x00, 0x01];
    pub const OPEN_ENDPOINT: [u8; 2] = [0x0D, 0x00];
    pub const CLOSE_ENDPOINT: [u8; 2] = [0x05, 0x01];
    pub const WRITE_FIRST: [u8; 2] = [0x06, 0x00];
    pub const WRITE_REST: [u8; 2] = [0x07, 0x00];
    pub const READ: [u8; 2] = [0x08, 0x00];
    /// RGB data endpoint (SetDirectColor)
    pub const RGB_ENDPOINT: [u8; 2] = [0x22, 0x00];
    pub const RGB_DATA_TYPE: [u8; 2] = [0x12, 0x00];
    /// LED count endpoint (GetLedCounts)
    pub const LED_COUNT_ENDPOINT: [u8; 2] = [0x20, 0x00];
    /// fan mode endpoint (SetFanMode)
    pub const FAN_ENDPOINT: [u8; 2] = [0x1E, 0x00];
    pub const FAN_DATA_TYPE: [u8; 2] = [0x0D, 0x00];
    pub const LED_AMOUNT_ENDPOINT: [u8; 2] = [0x1D, 0x00];
    pub const LED_AMOUNT_DATA_TYPE: [u8; 2] = [0x0C, 0x00];
}
/// upstream: SetFanMode() 15-byte payload for the Commander Core
/// (`07 01 08` header then `01 06` pairs from index 3 to 14).
pub fn fan_mode_packet(pid: u16, external_rgb_port: bool) -> Vec<u8> {
    let mut buf = vec![0u8; 15];
    buf[0] = 0x07;
    let (start, max) = if pid == PID_CORE_XT {
        if external_rgb_port {
            buf[1] = 0x01;
            buf[2] = 0x01;
            (3, 15)
        } else {
            (2, 14)
        }
    } else {
        buf[1] = 0x01;
        buf[2] = 0x08;
        (3, 15)
    };
    let mut i = start;
    while i < max {
        buf[i] = 0x01;
        buf[i + 1] = 0x06;
        i += 2;
    }
    buf
}
/// upstream: SetLedAmount() 15-byte payload (`07 <amount>`).
pub fn led_amount_packet(led_amount: u8) -> Vec<u8> {
    let mut buf = vec![0u8; 15];
    buf[0] = 0x07;
    buf[1] = led_amount;
    buf
}
/// upstream: GetLedCounts() - command_res bytes from the read answer;
/// device count at res[2], counts as little-endian words at res[5], res[9], ...
pub fn led_counts_from_response(res: &[u8]) -> Vec<usize> {
    let count = res[2] as usize;
    (0..count)
        .map(|i| ((res[i * 4 + 6] as usize) << 8) | res[i * 4 + 5] as usize)
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    const A: Rgb = Rgb { r: 1, g: 2, b: 3 };
    const B: Rgb = Rgb { r: 4, g: 5, b: 6 };
    // upstream: CorsairCommanderCoreController.cpp SendCommand()
    #[test]
    fn command_packet_is_byte_exact() {
        let p = command_packet(PACKET_V2, endpoints::VERSION, None);
        assert_eq!(p.len(), 97);
        assert_eq!(&p[0..4], &[0x00, 0x08, 0x02, 0x13]);
        assert!(p[4..].iter().all(|b| *b == 0));
        let q = command_packet(PACKET_V2, endpoints::WAKE_HW, Some(&[0x00, 0x02]));
        assert_eq!(&q[0..6], &[0x00, 0x08, 0x01, 0x03, 0x00, 0x02]);
        assert!(q[6..].iter().all(|b| *b == 0));
    }
    // upstream: CorsairCommanderCoreController.cpp WriteData() first packet
    #[test]
    fn data_packet_is_byte_exact() {
        let p = data_packet(endpoints::RGB_DATA_TYPE, &[0x11, 0x22], 100);
        // real_len = 100 + 2 = 102 = 0x66
        assert_eq!(p, vec![0x66, 0x00, 0x00, 0x00, 0x12, 0x00, 0x11, 0x22]);
    }
    // upstream: CorsairCommanderCoreController.cpp SetFanMode()
    #[test]
    fn fan_mode_packet_is_byte_exact() {
        let p = fan_mode_packet(PID_CORE, false);
        assert_eq!(
            &p[0..15],
            &[
                0x07, 0x01, 0x08, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01,
                0x06
            ]
        );
        let xt = fan_mode_packet(PID_CORE_XT, false);
        assert_eq!(
            &xt[0..15],
            &[
                0x07, 0x00, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06,
                0x00
            ]
        );
        let ext = fan_mode_packet(PID_CORE_XT, true);
        assert_eq!(&ext[0..4], &[0x07, 0x01, 0x01, 0x01]);
        assert_eq!(
            &ext[4..15],
            &[0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06, 0x01, 0x06]
        );
    }
    // upstream: CorsairCommanderCoreController.cpp SetLedAmount()
    #[test]
    fn led_amount_packet_is_byte_exact() {
        let p = led_amount_packet(30);
        assert_eq!(&p[0..2], &[0x07, 30]);
        assert!(p[2..].iter().all(|b| *b == 0));
    }
    // upstream: CorsairCommanderCoreController.cpp GetLedCounts()
    #[test]
    fn led_counts_response_is_parsed_upstream_exact() {
        let mut res = vec![0u8; 93];
        res[2] = 3;
        // count 34 as [0x22, 0x00], 24 as [0x18, 0x00], 8 as [0x08, 0x00]
        res[5] = 0x22;
        res[6] = 0x00;
        res[9] = 0x18;
        res[10] = 0x00;
        res[13] = 0x08;
        res[14] = 0x00;
        assert_eq!(led_counts_from_response(&res), vec![34, 24, 8]);
    }
    // upstream: CorsairCommanderCoreController.cpp SetDirectColor()
    #[test]
    fn rgb_buffer_packs_zones_and_padding() {
        // Pump 4 LEDs, two fan zones of 2 LEDs each -> no padding needed
        let counts = [4, 2, 2];
        let colors = vec![A, B, A, B, A, B, A, B];
        let (buf, off) = rgb_buffer(false, &counts, &colors);
        assert_eq!(buf.len(), RGB_DATA_LENGTH);
        // upstream pads every zone except the first to 34 LEDs
        assert_eq!(off, 8 * 3 + 2 * 3 * 32);
        assert_eq!(&buf[0..6], &[1, 2, 3, 4, 5, 6]);
        assert_eq!(&buf[12..18], &[1, 2, 3, 4, 5, 6]);
        // Pump zone with only 1 color set and an empty fan zone pads to 34
        let counts = [1, 0];
        let colors = vec![A];
        let (buf, off) = rgb_buffer(false, &counts, &colors);
        assert_eq!(off, 3 + 34 * 3);
        assert_eq!(&buf[0..3], &[1, 2, 3]);
        assert_eq!(buf[102], 0);
    }
}
