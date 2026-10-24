// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_luxafor::devices_table::{PID, VID};
fn main() {
    if std::env::args().any(|a| a == "--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Luxafor Flag","vid":VID,"pid":PID,"kind":"accessory","interface":null,"usage_page":null,"leds":6}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_luxafor::Luxafor::default());
}
