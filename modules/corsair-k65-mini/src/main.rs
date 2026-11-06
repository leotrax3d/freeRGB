// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Corsair K65 Mini","vid":0x1B1Cu16,"pid":0x1BAFu16,"kind":"keyboard","leds":64}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_corsair_k65_mini::CorsairK65Mini::default());
}
