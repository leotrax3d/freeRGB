// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream packet IDs from `ZalmanZSyncController.h`.
pub const PACKET_ID_FIRMWARE: u8 = 0x02;
pub const PACKET_ID_DIRECT: u8 = 0x32;
pub const PACKET_ID_COMMIT: u8 = 0x33;
pub const PACKET_ID_RESET: u8 = 0x37;
pub const PACKET_ID_PORT_STATE: u8 = 0x38;
pub const PORT_STATE_SOFTWARE: u8 = 0x02;

/// Upstream: ZalmanZSyncController.cpp SetChannelLEDs sends 50 LEDs per packet.
pub const LED_PER_PACKET: usize = 50;

/// Upstream: ZalmanZSyncController.cpp SendFirmwareRequest. 65-byte
/// `00 02` report, then a 17-byte status read; bytes 0x02/0x03 hold the
/// firmware version.
pub fn firmware_request() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = PACKET_ID_FIRMWARE;
    p
}

/// Firmware version string from a 17-byte response; upstream maps a byte
/// 0x03 below 112 onto the fixed "0.7.1" string.
pub fn firmware_string(r: &[u8]) -> Option<String> {
    if r.len() > 3 && r[3] < 112 {
        Some("0.7.1".into())
    } else if r.len() > 3 {
        Some(format!("{}.{}.{}", r[2], r[3] >> 4, r[3] & 0x0f))
    } else {
        None
    }
}

/// Upstream: ZalmanZSyncController.cpp SendPortState (software state 0x02).
pub fn state(channel: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = PACKET_ID_PORT_STATE;
    p[2] = channel;
    p[3] = PORT_STATE_SOFTWARE;
    p
}

/// Upstream: ZalmanZSyncController.cpp SendDirect. One 65-byte report per
/// colour plane: `00 32 <channel> <start> <count> <plane>` plus the plane
/// bytes.
pub fn direct(channel: u8, start: u8, colors: &[Rgb], plane: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = PACKET_ID_DIRECT;
    p[2] = channel;
    p[3] = start;
    p[4] = colors.len() as u8;
    p[5] = plane;
    for (i, c) in colors.iter().enumerate() {
        p[6 + i] = match plane {
            0 => c.r,
            1 => c.g,
            _ => c.b,
        };
    }
    p
}

/// Upstream: ZalmanZSyncController.cpp SendCommit (`00 33 FF`).
pub fn commit() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = PACKET_ID_COMMIT;
    p[2] = 0xff;
    p
}

/// Upstream: ZalmanZSyncController.cpp SendReset (`00 37 <channel>`), part of
/// the effect path only; kept as a tested builder for completeness.
pub fn reset(channel: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = PACKET_ID_RESET;
    p[2] = channel;
    p
}
