// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: GigabyteAorusLaptopController.cpp SetDirect:
/// feature report of REPORT_SIZE+1 (9) bytes: [0]=0 report id, [1]=0x08
/// (GIGABYTE_AORUS_LAPTOP_REPORT_ID), [2]=0x01 (direct mode), RGB at 3..5,
/// brightness at 6, byte 7 = 0, byte 8 = checksum (0 - bytes 1..7).
pub const REPORT_LEN: usize = 9;
pub fn direct(c: Rgb, brightness: u8) -> Vec<u8> {
    let mut buf = vec![0u8; REPORT_LEN];
    buf[1] = 0x08;
    buf[2] = 0x01;
    buf[3] = c.r;
    buf[4] = c.g;
    buf[5] = c.b;
    buf[6] = brightness;
    let mut checksum: u8 = 0;
    for b in &buf[1..8] {
        checksum = checksum.wrapping_sub(*b);
    }
    buf[8] = checksum;
    buf
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        // upstream: GigabyteAorusLaptopController.cpp SetDirect with
        // RGB(1,2,3), brightness 0x19 -> checksum = 0 - 0x08 - 0x01 - 1 - 2 - 3 -
        // 0x19 = 0xD8 (wrapping).
        let p = direct(Rgb { r: 1, g: 2, b: 3 }, 0x19);
        assert_eq!(&p[..9], &[0, 0x08, 0x01, 1, 2, 3, 0x19, 0, 0xd8]);
    }
}
