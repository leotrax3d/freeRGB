// SPDX-License-Identifier: GPL-2.0-or-later
//
// Razer 90-byte feature report, ported from OpenRGB
// Controllers/RazerController/RazerController/RazerController.{h,cpp}
// (razer_report, razer_calculate_crc, razer_create_* functions).
// Copyright (C) 2021 Adam Honse (CalcProgrammer1) and OpenRGB contributors.
// Modified for freeRGB: Rust port; only the report builders used by this
// module's Direct-mode path are carried; values and layout are unchanged.

/// Report ID byte + 90 report bytes, exactly as hidapi sends them.
pub const REPORT_LEN: usize = 91;

pub const STATUS_NEW: u8 = 0x00;
pub const STATUS_BUSY: u8 = 0x01;
pub const STATUS_SUCCESS: u8 = 0x02;

pub const STORAGE_NO_SAVE: u8 = 0x00;
pub const DEVICE_MODE_SOFTWARE: u8 = 0x03;
pub const COMMAND_GET_FIRMWARE_VERSION: u8 = 0x81;

// Field offsets inside the packed C struct (report_id first).
const STATUS: usize = 1;
const TRANSACTION: usize = 2;
const DATA_SIZE: usize = 6;
const CLASS: usize = 7;
const COMMAND: usize = 8;
pub const ARGS: usize = 9;
const CRC: usize = 89;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report(pub [u8; REPORT_LEN]);

impl Report {
    /// razer_create_report: zeroed, remaining_packets/protocol_type 0.
    pub fn new(report_id: u8, transaction_id: u8, class: u8, command: u8, data_size: u8) -> Self {
        let mut r = [0u8; REPORT_LEN];
        r[0] = report_id;
        r[TRANSACTION] = transaction_id;
        r[DATA_SIZE] = data_size;
        r[CLASS] = class;
        r[COMMAND] = command;
        Self(r)
    }

    pub fn args_mut(&mut self) -> &mut [u8] {
        &mut self.0[ARGS..ARGS + 80]
    }

    pub fn args(&self) -> &[u8] {
        &self.0[ARGS..ARGS + 80]
    }

    pub fn status(&self) -> u8 {
        self.0[STATUS]
    }

    /// XOR of bytes 3..89 (offset by the report ID byte, as upstream notes).
    pub fn crc(&self) -> u8 {
        self.0[3..89].iter().fold(0, |crc, b| crc ^ b)
    }

    /// razer_usb_send fills the CRC right before sending.
    pub fn sealed(mut self) -> Self {
        self.0[CRC] = self.crc();
        self
    }

    pub fn answers(&self, request: &Report) -> bool {
        self.0[TRANSACTION] == request.0[TRANSACTION]
            && self.0[CLASS] == request.0[CLASS]
            && self.0[COMMAND] == request.0[COMMAND]
            && self.status() != STATUS_NEW
            && self.status() != STATUS_BUSY
    }
}

/// The parameters every builder needs from the controller.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub report_id: u8,
    pub transaction_id: u8,
    pub led_id: u8,
}

impl Target {
    fn report(&self, class: u8, command: u8, size: u8) -> Report {
        Report::new(self.report_id, self.transaction_id, class, command, size)
    }

    pub fn device_mode(&self, mode: u8, param: u8) -> Report {
        let mut r = self.report(0x00, 0x04, 0x02);
        r.args_mut()[..2].copy_from_slice(&[mode, param]);
        r
    }

    pub fn get_firmware(&self) -> Report {
        self.report(0x00, COMMAND_GET_FIRMWARE_VERSION, 0x02)
    }

    pub fn brightness_standard(&self, brightness: u8) -> Report {
        let mut r = self.report(0x03, 0x03, 0x03);
        r.args_mut()[..3].copy_from_slice(&[STORAGE_NO_SAVE, self.led_id, brightness]);
        r
    }

    pub fn brightness_extended(&self, brightness: u8) -> Report {
        let mut r = self.report(0x0F, 0x04, 0x03);
        r.args_mut()[..3].copy_from_slice(&[STORAGE_NO_SAVE, self.led_id, brightness]);
        r
    }

    pub fn frame_standard(&self, row: u8, start: u8, stop: u8, rgb: &[u8]) -> Report {
        let len = (stop as usize + 1 - start as usize) * 3;
        let mut r = self.report(0x03, 0x0B, (len + 4) as u8);
        let a = r.args_mut();
        a[..4].copy_from_slice(&[0xFF, row, start, stop]);
        a[4..4 + len].copy_from_slice(&rgb[..len]);
        r
    }

    pub fn frame_extended(&self, row: u8, start: u8, stop: u8, rgb: &[u8]) -> Report {
        let len = (stop as usize + 1 - start as usize) * 3;
        let mut r = self.report(0x0F, 0x03, (len + 5) as u8);
        let a = r.args_mut();
        a[2..5].copy_from_slice(&[row, start, stop]);
        a[5..5 + len].copy_from_slice(&rgb[..len]);
        r
    }

    pub fn frame_linear(&self, start: u8, stop: u8, rgb: &[u8]) -> Report {
        let len = (stop as usize + 1 - start as usize) * 3;
        let mut r = self.report(0x03, 0x0C, 0x32);
        let a = r.args_mut();
        a[..2].copy_from_slice(&[start, stop]);
        a[2..2 + len].copy_from_slice(&rgb[..len]);
        r
    }

    pub fn mode_custom_standard(&self) -> Report {
        let mut r = self.report(0x03, 0x0A, 0x02);
        r.args_mut()[..2].copy_from_slice(&[0x05, STORAGE_NO_SAVE]);
        r
    }

    pub fn mode_custom_extended(&self) -> Report {
        let mut r = self.report(0x0F, 0x02, 0x0C);
        r.args_mut()[..3].copy_from_slice(&[0x00, 0x00, 0x08]);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COBRA: Target = Target {
        report_id: 0,
        transaction_id: 0x1F,
        led_id: 0x05,
    };

    /// Expected bytes worked out by hand from the upstream C++ builders.
    #[test]
    fn extended_frame_and_custom_mode_match_upstream_layout() {
        let r = COBRA.frame_extended(0, 0, 0, &[0x11, 0x22, 0x33]).sealed();
        assert_eq!(
            r.0[..14],
            [0, 0, 0x1F, 0, 0, 0, 0x08, 0x0F, 0x03, 0, 0, 0, 0, 0]
        );
        assert_eq!(r.0[14..17], [0x11, 0x22, 0x33]);
        assert_eq!(r.0[89], 0x08 ^ 0x0F ^ 0x03 ^ 0x11 ^ 0x22 ^ 0x33);
        assert_eq!(r.0[90], 0);
        let m = COBRA.mode_custom_extended().sealed();
        assert_eq!(m.0[6..12], [0x0C, 0x0F, 0x02, 0x00, 0x00, 0x08]);
        assert_eq!(m.0[89], 0x0C ^ 0x0F ^ 0x02 ^ 0x08);
    }

    #[test]
    fn standard_and_linear_builders_keep_upstream_sizes() {
        let rgb = [1u8; 9];
        let s = COBRA.frame_standard(2, 0, 2, &rgb);
        assert_eq!(s.0[6..13], [13, 0x03, 0x0B, 0xFF, 2, 0, 2]);
        let l = COBRA.frame_linear(0, 2, &rgb);
        assert_eq!(l.0[6..11], [0x32, 0x03, 0x0C, 0, 2]);
        let b = COBRA.brightness_extended(255);
        assert_eq!(b.0[6..12], [3, 0x0F, 0x04, 0, 0x05, 255]);
    }

    #[test]
    fn response_matching_requires_final_status() {
        let request = COBRA.get_firmware();
        let mut reply = request.clone();
        reply.0[1] = STATUS_BUSY;
        assert!(!reply.answers(&request));
        reply.0[1] = STATUS_SUCCESS;
        assert!(reply.answers(&request));
        reply.0[8] = 0x82;
        assert!(!reply.answers(&request));
    }
}
