//! Synthetic API 2 process fixture; enabled only with bus-fixture.
use freergb_core::{LedLayout, Rgb};
use freergb_module_api::bus::{
    self,
    sdk::{Bus, Module},
    Operation, PciId, Transfer,
};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, ModuleError, ModuleInfo, Restore,
};

struct Fixture {
    mode: String,
}
fn error() -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, "fixture bus transaction failed")
}
fn write(register: u8, data: Vec<u8>) -> Transfer {
    Transfer {
        op: Operation::I2cWriteRegister,
        address: 0x18,
        register,
        data,
        read_len: 0,
    }
}
impl Module for Fixture {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: if self.mode == "wrong-identity" {
                "other"
            } else {
                "bus-fixture"
            }
            .into(),
            version: "0.0.1".into(),
            family: "bus-fixture".into(),
            api: if self.mode == "old-api" { 1 } else { bus::API },
        }
    }
    fn identify(&self, _: PciId) -> Identification {
        Identification {
            supported: self.mode != "refuse",
            model: Some("Synthetic bus fixture".into()),
            kind: Some(DeviceKind::LightingController),
            reason: None,
        }
    }
    fn initialize(&mut self, _: PciId, bus: &mut dyn Bus) -> Result<Capabilities, ModuleError> {
        bus.transaction(vec![Transfer {
            op: Operation::SmbusReadByteData,
            address: 0x18,
            register: 0,
            data: vec![],
            read_len: 1,
        }])
        .map_err(|_| error())?;
        Ok(Capabilities {
            name: "Synthetic bus fixture".into(),
            kind: DeviceKind::LightingController,
            layout: LedLayout::strip(2),
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], bus: &mut dyn Bus) -> Result<(), ModuleError> {
        match self.mode.as_str() {
            "wrong-request" | "wrong-version" => {
                let envelope = bus::ProcessCall {
                    request: if self.mode == "wrong-request" { 3 } else { 4 },
                    transaction: bus::Call {
                        v: if self.mode == "wrong-version" { 1 } else { 2 },
                        call: 2,
                        capability: 0,
                        transfers: vec![write(0x20, vec![1])],
                    },
                };
                println!("{}", serde_json::to_string(&envelope).unwrap());
                loop {
                    std::thread::park();
                }
            }
            "crash" => std::process::exit(3),
            "hang" => loop {
                std::thread::park();
            },
            "oversize" => {
                use std::io::Write;
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(&vec![b' '; freergb_module_api::MAX_MESSAGE + 1]);
                let _ = out.flush();
                loop {
                    std::thread::park();
                }
            }
            "malformed" => {
                println!("{{bad json");
                loop {
                    std::thread::park();
                }
            }
            _ => {}
        }
        let mut request = write(0x20, colors.iter().flat_map(|c| [c.r, c.g, c.b]).collect());
        if self.mode == "forbidden" || self.mode == "hide-denial" {
            request.address = 0x50;
        }
        let result = bus.transaction(vec![request]);
        if self.mode == "crash-after-write" {
            std::process::exit(3);
        }
        if self.mode == "hide-denial" {
            return Ok(());
        }
        result.map(|_| ()).map_err(|_| error())
    }
    fn shutdown(&mut self, bus: &mut dyn Bus) -> Result<(), ModuleError> {
        bus.transaction(vec![write(0x10, vec![0])])
            .map(|_| ())
            .map_err(|_| error())
    }
}
fn main() {
    let mode = std::fs::read_to_string("fixture-mode").unwrap_or_else(|_| "ok".into());
    if mode == "io-hello" {
        use std::io::{BufRead, Write};
        let mut request = String::new();
        let _ = std::io::stdin().lock().read_line(&mut request);
        let envelope = bus::ProcessCall {
            request: 1,
            transaction: bus::Call {
                v: 2,
                call: 1,
                capability: 0,
                transfers: vec![write(0x20, vec![1])],
            },
        };
        println!("{}", serde_json::to_string(&envelope).unwrap());
        let _ = std::io::stdout().flush();
        loop {
            std::thread::park();
        }
    }
    bus::sdk::run(Fixture { mode });
}
