//! Explicit offline diagnostic export with stable device aliases. No raw errors,
//! executable paths, serial numbers, macro contents or capture names are included.
use crate::engine::Engine;
use freergb_device_api::DeviceStatus;
use serde_json::{json, Value};

fn status(status: &DeviceStatus) -> &'static str {
    match status {
        DeviceStatus::Connected => "connected",
        DeviceStatus::Disconnected => "disconnected",
        DeviceStatus::Faulted(_) => "faulted",
    }
}
pub fn create(engine: &Engine) -> Value {
    let diagnostics = engine.diagnostics();
    let devices:Vec<_>=engine.devices().iter().enumerate().map(|(index,device)| json!({
        "device":index+1,"vendor":device.info.vendor,"kind":device.info.kind,
        "usb_vid":device.info.usb_vid,"usb_pid":device.info.usb_pid,
        "status":status(&device.status),"leds":device.leds,"frames":device.frames,"dropped":device.dropped,
        "has_error":device.last_error.is_some(),
        "sensors":device.sensors.iter().enumerate().map(|(index,sensor)|json!({"sensor":index+1,"unit":sensor.unit,"value":sensor.value,"age_ms":sensor.age_ms})).collect::<Vec<_>>(),
    })).collect();
    let plugins:Vec<_>=engine.plugins().iter().enumerate().map(|(index,plugin)| json!({
        "plugin":index+1,"access":plugin.access,"enabled":plugin.enabled,"allowed":plugin.allowed,
        "status":plugin.status.as_ref().map(status),"frames":plugin.frames,"has_error":plugin.last_error.is_some(),
    })).collect();
    let cooling: Vec<_> = diagnostics
        .cooling_support
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            json!({
                "channel":index+1,"tier":entry.tier,"register_readback":entry.register_readback,
                "software_mode":entry.software_mode,"held_full":entry.held_full,
            })
        })
        .collect();
    json!({"report_version":1,"application_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,
        "runtime":{"uptime_s":diagnostics.uptime_s,"ticks":diagnostics.ticks,"target_fps":diagnostics.target_fps,"measured_fps":diagnostics.measured_fps,"brightness":diagnostics.brightness,"effect":diagnostics.effect.name(),"conflicts":diagnostics.conflicts},
        "devices":devices,"plugins":plugins,"cooling_channels":cooling,
        "redaction":"Device IDs/names/serials, raw errors, capture names, paths and macro/profile contents omitted. Review before sharing."})
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::{LedLayout, Rgb};
    use freergb_device_api::VirtualDevice;
    #[test]
    fn exported_report_omits_custom_names_and_persisted_paths() {
        let mut engine = Engine::new(freergb_effects::EffectSpec::Solid { color: Rgb::RED }, 30);
        engine.add_device(Box::new(VirtualDevice::new(
            "PRIVATE-DEVICE-NAME",
            LedLayout::strip(1),
        )));
        engine.controls.call("macros.save",&json!({"name":"PRIVATE-MACRO-NAME","definition":{"repeat":1,"steps":[{"type":"text","text":"PRIVATE-MACRO-CONTENT"}]}})).unwrap();
        let report = create(&engine);
        let text = report.to_string();
        assert_eq!(report["report_version"], 1);
        assert_eq!(report["devices"][0]["leds"], 1);
        assert!(!text.contains("PRIVATE") && !text.contains("virtual:"));
        assert!(report["devices"][0].get("serial").is_none());
        assert!(report["devices"][0].get("id").is_none());
    }
}
