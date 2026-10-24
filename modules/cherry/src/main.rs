// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().any(|a| a == "--list-devices") {
        println!("{}", serde_json::to_string_pretty(&freergb_module_cherry::devices_table::DETECTORS.iter().map(|d| serde_json::json!({"name": d.name, "vid": d.vid, "pid": d.pid, "kind": "keyboard"})).collect::<Vec<_>>()).unwrap());
        return;
    }
    freergb_module_api::sdk::run(freergb_module_cherry::Cherry::default());
}
