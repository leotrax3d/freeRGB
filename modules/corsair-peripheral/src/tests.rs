// SPDX-License-Identifier: GPL-2.0-or-later
#[test]
fn packets_are_byte_exact() {
    let p = crate::report::mouse(&[freergb_core::Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..9], &[0, 7, 13, 1, 0, 0, 1, 2, 3]);
    let k = crate::report::k65(&[freergb_core::Rgb { r: 4, g: 5, b: 6 }]);
    assert_eq!(&k[1..9], &[8, 6, 1, 0x73, 1, 0, 0, 0x12]);
}
#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 39);
}
