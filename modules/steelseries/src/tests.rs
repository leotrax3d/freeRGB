// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1038,
        pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
#[test]
fn inventory_is_complete() {
    assert_eq!(devices_table::DETECTORS.len(), 76)
}
#[test]
fn inventoried_devices_are_not_claimed_before_protocol_port() {
    let result = crate::table::identify(&iface(devices_table::DETECTORS[0].pid));
    assert!(!result.supported);
    assert!(result.reason.unwrap().contains("protocol is not ported"));
}
