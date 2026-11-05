// SPDX-License-Identifier: GPL-2.0-or-later


fn main() {
    if std::env::args().nth(1).as_deref() == Some("--list-devices") {
        let devices = vec![serde_json::json!({
            "name": "Dream Cheeky Webmail Notifier",
            "vid": 0x1D34u16,
            "pid": 0x0004u16,
            "kind": "accessory"
        })];
        println!("{}", serde_json::to_string_pretty(&devices).unwrap());
        return;
    }
    freergb_module_api::sdk::run(freergb_module_dream_cheeky::DreamCheeky::default());
}