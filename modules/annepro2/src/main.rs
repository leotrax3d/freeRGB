// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().any(|a| a == "--list-devices") {
        println!(
            "{}",
            serde_json::json!({"module":freergb_module_annepro2::MODULE_ID,"devices":freergb_module_annepro2::devices_table::DETECTORS.iter().map(|d|serde_json::json!({"name":d.name,"vid":format!("{:04x}",d.vid),"pid":format!("{:04x}",d.pid)})).collect::<Vec<_>>()})
        );
    }
}
