// SPDX-License-Identifier: GPL-2.0-or-later
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        println!(
            "{}",
            serde_json::json!([{"name":"Corsair K55 RGB PRO XT","vid":0x1B1Cu16,"pid":0x1BA1u16,"kind":"keyboard","leds":111}])
        );
        return;
    }
    freergb_module_api::sdk::run(freergb_module_corsair_k55_pro_xt::CorsairK55ProXt::default());
}
