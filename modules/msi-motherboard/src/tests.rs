// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1462,
        pid,
        interface: 1,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
#[test]
fn generated_inventory_has_all_upstream_detectors() {
    assert_eq!(devices_table::DETECTORS.len(), 124);
}

#[test]
fn inventoried_devices_are_not_claimed_before_lifecycle_port() {
    let result = crate::table::identify(&iface(0x1720));
    assert!(!result.supported);
    assert!(result.reason.unwrap().contains("lifecycle is not ported"));
}
