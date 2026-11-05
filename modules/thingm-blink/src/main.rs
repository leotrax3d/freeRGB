// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_thingm_blink::devices_table::{PID, USAGE, USAGE_PAGE, VID};
fn main() {
    if std::env::args().any(|a| a == "--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"ThingM blink(1) mk2","vid":VID,"pid":PID,"interface":null,"usage_page":USAGE_PAGE,"usage":USAGE,"kind":"lighting_controller","leds":2}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_thingm_blink::ThingmBlink::default());
}
