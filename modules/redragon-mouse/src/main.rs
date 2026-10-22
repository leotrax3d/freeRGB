// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_redragon_mouse::{devices_table::DEVICES, table};
fn main() {
    if std::env::args().any(|a| a == "--list-devices") {
        let out: Vec<_> = DEVICES.iter().map(|(name, pid)| serde_json::json!({"name": name, "vid": 0x04D9u16, "pid": pid, "interface": 2, "usage_page": 0xFFA0u16, "kind": "mouse", "leds": 1})).collect();
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        return;
    }
    let _ = table::identify;
    freergb_module_api::sdk::run(freergb_module_redragon_mouse::RedragonMouse::default());
}
