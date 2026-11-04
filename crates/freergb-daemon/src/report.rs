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

/// Opt-in fan-support evidence: board model, BIOS, chip, tier, coverage and
/// channel-test outcome per anonymous control. No serials, names, readings or
/// paths. This is the payload of the "Fan control hardware report" issue
/// template; `scripts/cooling/import-report.mjs` turns it into matrix evidence.
pub fn cooling_hardware(engine: &Engine) -> Value {
    let summary = engine.mainboard.summary();
    let channels = engine.mainboard.channels();
    let support = engine.fan_support.view(summary.board.as_ref(), &channels);
    let controls: Vec<_> = channels
        .iter()
        .filter(|c| c.kind == "control" && c.control_available)
        .enumerate()
        .map(|(index, c)| {
            let chip = crate::fan_coverage::chip_of(&c.sensor_id).unwrap_or("unknown");
            let backend = if c.provider == crate::hwmon::PROVIDER {
                "hwmon"
            } else {
                "lhm"
            };
            let evidence = &support["channels"][c.sensor_id.as_str()];
            json!({
                "channel": index + 1,
                "chip": chip,
                "backend": backend,
                "coverage": crate::fan_coverage::lookup(&c.sensor_id).map(|v| v["status"].clone()),
                "tier": c.support.as_ref().map_or("unknown", |s| s.tier.as_str()),
                "register_readback": c.support.as_ref().is_some_and(|s| s.register_readback),
                "test": evidence["test"],
                "tested": evidence["tested"],
                "unlocked": evidence["unlock"].as_str().is_some_and(|u| u.starts_with("unlocked")),
            })
        })
        .collect();
    json!({
        "report_format": "freergb-fan-report/1",
        "application_version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "board": summary.board.map(|b| json!({"manufacturer": b.manufacturer, "product": b.product, "bios_version": b.bios_version})),
        "lhm_version": freergb_core::lhm::VERSION,
        "controls": controls,
    })
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
        assert!(
            report.get("cooling_hardware").is_none(),
            "cooling hardware is opt-in"
        );
        let cooling = cooling_hardware(&engine);
        assert_eq!(cooling["controls"], json!([]));
        assert_eq!(cooling["lhm_version"], freergb_core::lhm::VERSION);
    }
}

/// Explicit error-report preview. Excludes raw errors, serials, names, paths,
/// sensor readings and anonymous-rating credentials. Nothing is sent here.
pub fn marketplace(engine: &Engine, kind: &str) -> Result<Value, String> {
    let (code, effect) = match kind {
        "runtime" => {
            let code = engine
                .runtime_error_code()
                .ok_or("There is no current effect error to report")?;
            let effect = match engine.effect() {
                freergb_effects::EffectSpec::Marketplace { id, version, .. } => {
                    Some(json!({"id":id,"version":version}))
                }
                _ => None,
            };
            (code.to_owned(), effect)
        }
        "operation" => {
            let manager = engine
                .marketplace
                .as_ref()
                .ok_or("Marketplace unavailable")?;
            let failure = manager
                .view()
                .failure
                .ok_or("There is no current marketplace error to report")?;
            (
                failure.code,
                Some(json!({"id":failure.effect_id,"version":failure.version})),
            )
        }
        _ => return Err("Choose a current runtime or marketplace operation error".into()),
    };
    let diagnostic = create(engine);
    let all = diagnostic["devices"]
        .as_array()
        .ok_or("Devices unavailable")?;
    let devices:Vec<_>=all.iter().take(12).map(|d|json!({
        "vendor":d["vendor"].as_str().map(|s|s.chars().take(64).collect::<String>()),
        "kind":d["kind"],"usb_vid":d["usb_vid"],"usb_pid":d["usb_pid"],"status":d["status"],"leds":d["leds"],"has_error":d["has_error"],
    })).collect();
    let payload = json!({"report_version":1,"error_code":code,"effect":effect,
        "application_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,
        "devices":devices,"additional_devices":all.len().saturating_sub(12)});
    let title = format!("[Effect error] {code}");
    let body=format!("## Error report\n\n```json\n{}\n```\n\n## What happened?\nPlease describe the steps that led to the error.\n\nThis report omits device names, serial numbers, paths, sensor readings, raw error messages and rating-session credentials.\n",serde_json::to_string_pretty(&payload).map_err(|e|e.to_string())?);
    Ok(json!({"code":code,"title":title,"body":body,"payload":payload}))
}

#[cfg(test)]
mod marketplace_tests {
    use super::*;
    use freergb_core::{LedLayout, Rgb};
    use freergb_device_api::VirtualDevice;
    #[test]
    fn report_requires_a_real_error_and_excludes_private_details() {
        let mut engine = Engine::new(freergb_effects::EffectSpec::Solid { color: Rgb::BLACK }, 30);
        assert!(marketplace(&engine, "runtime").is_err());
        engine.add_device(Box::new(VirtualDevice::new(
            "PRIVATE-DEVICE-NAME",
            LedLayout::strip(2),
        )));
        engine.set_effect(freergb_effects::EffectSpec::Marketplace {
            id: "aurora".into(),
            version: "1.0.0".into(),
            parameters: Default::default(),
        });
        let report = marketplace(&engine, "runtime").unwrap();
        assert_eq!(report["code"], "FX_ACTIVATION_FAILED");
        let body = report["body"].as_str().unwrap();
        assert!(body.contains("FX_ACTIVATION_FAILED"));
        assert!(!body.contains("PRIVATE-DEVICE-NAME"));
        assert!(!body.contains("virtual:"));
        assert!(report["payload"]["devices"][0].get("serial").is_none());
        assert!(report["payload"]["devices"][0].get("name").is_none());
        assert!(marketplace(&engine, "arbitrary-code").is_err());
    }
}
