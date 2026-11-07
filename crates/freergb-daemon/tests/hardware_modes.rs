//! Local RPC -> bounded worker queue -> simulated device, without hardware.
use freergb_core::{Frame, LedLayout, Rgb};
use freergb_daemon::{api, engine::Engine, lock, SharedEngine};
use freergb_device_api::{DeviceError, DeviceInfo, DeviceStatus, RgbDevice, VirtualDevice};
use freergb_effects::EffectSpec;
use freergb_module_api::modes::{self, Command};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

struct Simulated {
    device: VirtualDevice,
    commands: Arc<Mutex<Vec<Value>>>,
    state: modes::State,
}

impl RgbDevice for Simulated {
    fn info(&self) -> &DeviceInfo {
        self.device.info()
    }
    fn layout(&self) -> &LedLayout {
        self.device.layout()
    }
    fn status(&self) -> DeviceStatus {
        self.device.status()
    }
    fn connect(&mut self) -> Result<(), DeviceError> {
        self.device.connect()
    }
    fn disconnect(&mut self, color: Option<Rgb>) -> Result<(), DeviceError> {
        self.device.disconnect(color)
    }
    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        self.device.push_frame(frame)
    }
    fn hardware_mode_command(&mut self, value: Value) -> Result<Value, DeviceError> {
        let command: Command = serde_json::from_value(value.clone()).unwrap();
        self.state
            .check(&command, 2)
            .map_err(|e| DeviceError::Refused(e.to_string()))?;
        self.state.acknowledge(&command);
        self.commands.lock().unwrap().push(value);
        Ok(serde_json::to_value(&self.state).unwrap())
    }
}

fn rpc(engine: &SharedEngine, device: &str, command: Value) -> Value {
    serde_json::from_str(
        &api::handle_line(
            engine,
            &json!({"id":1,"method":"modules.modes", "params":{"device":device,"command":command}})
                .to_string(),
        )
        .line,
    )
    .unwrap()
}

#[test]
fn local_api_serializes_mode_commands_and_requires_separate_save_confirmation() {
    let commands = Arc::new(Mutex::new(Vec::new()));
    let simulated = Simulated {
        device: VirtualDevice::new("modes", LedLayout::strip(2)),
        commands: commands.clone(),
        state: modes::State {
            active: None,
            catalog: modes::Catalog {
                version: 1,
                modes: vec![modes::Mode {
                    id: "static".into(),
                    name: "Static".into(),
                    value: 0,
                    flags: modes::MODE_COLOR | modes::MANUAL_SAVE,
                    speed: None,
                    brightness: None,
                    colors: modes::Range { min: 1, max: 1 },
                }],
            },
        },
    };
    let id = simulated.info().id.0.clone();
    let engine: SharedEngine = Arc::new(Mutex::new(Engine::new(
        EffectSpec::Solid { color: Rgb::BLACK },
        30,
    )));
    lock(&engine).add_device(Box::new(simulated));
    // Worker connects before processing commands, including when no tick runs.
    assert_eq!(
        rpc(&engine, &id, json!({"op":"list"}))["result"]["catalog"]["version"],
        1
    );
    let selection = json!({"id":"static","color_type":2,"colors":[{"r":1,"g":2,"b":3}]});
    assert_eq!(
        rpc(&engine, &id, json!({"op":"set","selection":selection}))["result"]["active"]["id"],
        "static"
    );
    assert!(rpc(&engine, &id, json!({"op":"save","confirmed":false}))["error"].is_object());
    assert!(rpc(&engine, &id, json!({"op":"save"}))["error"].is_object());
    assert_eq!(commands.lock().unwrap().len(), 2);
    assert!(rpc(&engine, &id, json!({"op":"save","confirmed":true}))["error"].is_null());
    assert!(rpc(&engine, &id, json!({"op":"direct"}))["result"]["active"].is_null());
    lock(&engine).set_lighting_paused(true);
    assert!(rpc(&engine, &id, json!({"op":"save","confirmed":true}))["error"].is_object());
    assert!(rpc(&engine, "missing", json!({"op":"list"}))["error"].is_object());
    lock(&engine).shutdown();
}
