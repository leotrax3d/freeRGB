// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x0b05,
        pid,
        interface: -1,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
#[test]
fn inventory_excludes_builtin() {
    assert!(devices_table::DETECTORS.iter().all(|d| d.pid != 0x19af));
    assert_eq!(devices_table::DETECTORS.len(), 100);
}
#[test]
fn inventoried_devices_are_not_claimed_before_controller_port() {
    let result = crate::table::identify(&iface(0x1867));
    assert!(!result.supported);
    assert!(result
        .reason
        .unwrap()
        .contains("complete controller protocol"));
}
