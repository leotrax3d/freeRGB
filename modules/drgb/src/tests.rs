// SPDX-License-Identifier: GPL-2.0-or-later
#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 33);
    assert_eq!(crate::devices_table::DEVICES.len(), 33);
}
#[test]
fn v2_packet_is_byte_exact() {
    let p = crate::report::v2(&[freergb_core::Rgb { r: 1, g: 2, b: 3 }], 1, 1);
    assert_eq!(&p[0][1..7], &[1, 1, 0, 1, 2, 3]);
}

#[test]
fn configured_zone_sizes_are_encoded() {
    let packets = crate::report::v3_zoned(&[freergb_core::Rgb { r: 1, g: 2, b: 3 }], &[1, 7], 0x60);
    assert_eq!(&packets[0][2..6], &[0, 1, 0, 7]);
}
