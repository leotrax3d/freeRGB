// SPDX-License-Identifier: GPL-2.0-or-later
#[test]
fn generated_catalog_matches_upstream() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 25);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x046a));
}
