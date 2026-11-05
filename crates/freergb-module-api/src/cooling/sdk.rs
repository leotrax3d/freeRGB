//! Plugin side of API 3: implement [`CoolingPlugin`] and call [`run`].
//!
//! Device I/O goes through the same brokered [`Hid`] as hardware modules; the
//! host checks every call against the signed policy and faults the plugin on
//! the first denied one. On EOF (host gone) the SDK calls `shutdown`, which
//! must hand every channel back to firmware control.
use super::{Description, Method, Request, Sample, API};
use crate::sdk::{Hid, Host};
use crate::{ErrorCode, ModuleError, ModuleInfo, Response};
use std::io::{BufRead, Write};

pub trait CoolingPlugin {
    fn info(&self) -> ModuleInfo;
    /// Must equal the sensors and channels of the signed policy.
    fn describe(&self) -> Description;
    fn sample(&mut self, hid: &mut dyn Hid) -> Result<Sample, ModuleError>;
    /// The host renews the lease on every control tick; a plugin with its own
    /// timer should fall back to 100 % when `lease_ms` passes unrenewed.
    fn set_duty(
        &mut self,
        _channel: &str,
        _duty_percent: u8,
        _lease_ms: u64,
        _hid: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        Err(ModuleError::new(
            ErrorCode::Unsupported,
            "plugin has no controls",
        ))
    }
    fn release(&mut self, _channel: &str, _hid: &mut dyn Hid) -> Result<(), ModuleError> {
        Ok(())
    }
    fn shutdown(&mut self, hid: &mut dyn Hid) -> Result<(), ModuleError>;
}

fn protocol(message: &str) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

fn value<T: serde::Serialize>(v: T) -> Result<serde_json::Value, ModuleError> {
    serde_json::to_value(v).map_err(|e| protocol(&e.to_string()))
}

pub fn serve<P: CoolingPlugin, R: BufRead, W: Write>(plugin: &mut P, input: R, output: W) {
    let mut host = Host::with_version(input, output, API);
    loop {
        let line = match host.read_line() {
            Ok(Some(line)) => line,
            _ => {
                let _ = plugin.shutdown(&mut host);
                return;
            }
        };
        host.calls = 0;
        let Ok(request) = serde_json::from_slice::<Request>(&line) else {
            let _ = plugin.shutdown(&mut host);
            return;
        };
        let outcome = if request.v != API {
            Err(protocol("unsupported protocol version"))
        } else {
            match &request.method {
                Method::Hello => value(plugin.info()),
                Method::Describe => value(plugin.describe()),
                Method::Sample => plugin.sample(&mut host).and_then(value),
                Method::SetDuty {
                    channel,
                    duty_percent,
                    lease_ms,
                    ..
                } => plugin
                    .set_duty(channel, *duty_percent, *lease_ms, &mut host)
                    .and_then(|()| value(true)),
                Method::Release { channel } => plugin
                    .release(channel, &mut host)
                    .and_then(|()| value(true)),
                Method::Shutdown => plugin.shutdown(&mut host).and_then(|()| value(true)),
            }
        };
        let done = request.method == Method::Shutdown;
        let (result, error) = match outcome {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        };
        let sent = host.send_line(&Response {
            v: API,
            id: request.id,
            result,
            error,
        });
        if sent.is_err() || done {
            return;
        }
    }
}

pub fn run<P: CoolingPlugin>(mut plugin: P) {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(&mut plugin, stdin.lock(), stdout.lock());
}
