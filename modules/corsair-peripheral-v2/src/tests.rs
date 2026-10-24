// SPDX-License-Identifier: GPL-2.0-or-later
#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 37);
}
#[test]
fn packets_are_byte_exact() {
    let p = crate::report::direct(&[freergb_core::Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..7], &[8, 3, 1, 1, 1, 2, 3]);
    let packets = crate::report::block_packets(8, &[1, 2, 3]);
    assert_eq!(&packets[0][1..9], &[8, 0x30, 0, 3, 0, 0, 0, 1]);
    assert_eq!(&packets[1][1..4], &[8, 0x32, 1]);
}
