// SPDX-License-Identifier: GPL-2.0-or-later
#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 38);
}
#[test]
fn packets_are_byte_exact() {
    let p = crate::report::direct(&[freergb_core::Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..4], &[1, 1, 2, 3]);
    let m = crate::report::mm712_direct(freergb_core::Rgb { r: 4, g: 5, b: 6 });
    assert_eq!(&m[..7], &[0, 0x5a, 0x81, 3, 4, 5, 6]);
}
