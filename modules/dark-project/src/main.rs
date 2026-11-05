// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Dark Project KD3B V2","vid":0x195D,"pid":0x2061,"interface":2,"usage_page":0xFFC2,"usage":0x04,"kind":"keyboard","leds":87}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_dark_project::DarkProject::default());
}
