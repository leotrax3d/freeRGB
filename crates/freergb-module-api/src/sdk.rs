//! Module side of the protocol: implement [`Module`] and call [`serve`].
//!
//! All device I/O goes through [`Host`], which forwards each report to the
//! daemon and blocks for its answer. A module never opens a device itself.
use crate::*;
use std::io::{BufRead, Read, Write};

/// Brokered access to the single interface the host assigned.
pub trait Hid {
    fn write(&mut self, report: &[u8]) -> Result<usize, ModuleError>;
    fn read(&mut self, len: usize, timeout_ms: i32) -> Result<Vec<u8>, ModuleError>;
    fn send_feature(&mut self, report: &[u8]) -> Result<usize, ModuleError>;
    /// `prefix[0]` is the report ID; returns the report including the ID.
    fn get_feature(&mut self, prefix: &[u8], len: usize) -> Result<Vec<u8>, ModuleError>;
}

pub trait Module {
    fn info(&self) -> ModuleInfo;
    /// Table lookup only. Must not assume an identity it cannot prove.
    fn identify(&self, interface: &InterfaceInfo) -> Identification;
    /// Optional API-1 host metadata; old hosts omit it and old modules ignore it.
    fn identify_with_platform(
        &self,
        interface: &InterfaceInfo,
        _platform: Option<&PlatformIdentity>,
    ) -> Identification {
        self.identify(interface)
    }
    fn initialize(
        &mut self,
        interface: &InterfaceInfo,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError>;
    /// Initialize with optional user-provided zone sizes. The default keeps
    /// existing modules source- and wire-compatible.
    fn initialize_with_zones(
        &mut self,
        interface: &InterfaceInfo,
        _zones: Option<&[Zone]>,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize(interface, hid)
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError>;
    fn initialize_with_platform(
        &mut self,
        interface: &InterfaceInfo,
        zones: Option<&[Zone]>,
        _platform: Option<&PlatformIdentity>,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(interface, zones, hid)
    }
    fn status(&self) -> Status {
        Status {
            ok: true,
            detail: None,
        }
    }
    fn shutdown(&mut self, final_color: Option<Rgb>, hid: &mut dyn Hid) -> Result<(), ModuleError>;
}

/// Host transport over the module's own stdin/stdout.
pub struct Host<R, W> {
    input: R,
    output: W,
    next_call: u64,
    pub(crate) calls: u32,
    /// Version stamped on transport calls: 1, or 3 for cooling plugins.
    version: u32,
}

fn protocol(message: impl Into<String>) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

impl<R: BufRead, W: Write> Host<R, W> {
    pub fn new(input: R, output: W) -> Self {
        Self::with_version(input, output, MODULE_API)
    }

    pub(crate) fn with_version(input: R, output: W, version: u32) -> Self {
        Self {
            input,
            output,
            next_call: 0,
            calls: 0,
            version,
        }
    }

    pub(crate) fn send_line(&mut self, value: &impl Serialize) -> Result<(), ModuleError> {
        let mut line = serde_json::to_vec(value).map_err(|e| protocol(e.to_string()))?;
        if line.len() >= MAX_MESSAGE {
            return Err(protocol("message too large"));
        }
        line.push(b'\n');
        self.output
            .write_all(&line)
            .and_then(|_| self.output.flush())
            .map_err(|e| ModuleError::new(ErrorCode::Disconnected, e.to_string()))
    }

    /// Reads one line; `Ok(None)` on EOF.
    pub(crate) fn read_line(&mut self) -> Result<Option<Vec<u8>>, ModuleError> {
        let mut line = Vec::new();
        let n = (&mut self.input)
            .take((MAX_MESSAGE + 1) as u64)
            .read_until(b'\n', &mut line)
            .map_err(|e| ModuleError::new(ErrorCode::Disconnected, e.to_string()))?;
        if n == 0 {
            return Ok(None);
        }
        if line.len() > MAX_MESSAGE || line.last() != Some(&b'\n') {
            return Err(protocol("oversized or incomplete line"));
        }
        Ok(Some(line))
    }

    fn call(
        &mut self,
        op: Op,
        data: &[u8],
        len: usize,
        timeout_ms: i32,
    ) -> Result<CallReply, ModuleError> {
        if data.len() > MAX_REPORT || len > MAX_REPORT {
            return Err(protocol("report too large"));
        }
        self.calls += 1;
        if self.calls > MAX_CALLS_PER_REQUEST {
            return Err(protocol("too many transport calls in one request"));
        }
        self.next_call += 1;
        let call = Call {
            v: self.version,
            call: self.next_call,
            op,
            data: encode(data),
            len,
            timeout_ms,
        };
        self.send_line(&call)?;
        let line = self
            .read_line()?
            .ok_or_else(|| ModuleError::new(ErrorCode::Disconnected, "host closed the pipe"))?;
        let reply: CallReply =
            serde_json::from_slice(&line).map_err(|e| protocol(e.to_string()))?;
        if reply.v != self.version || reply.call != call.call {
            return Err(protocol("host reply version/call mismatch"));
        }
        if let Some(error) = reply.error {
            let code = if error.contains("disconnected") {
                ErrorCode::Disconnected
            } else if error.contains("timed out") {
                ErrorCode::Timeout
            } else {
                ErrorCode::Protocol
            };
            return Err(ModuleError::new(code, error));
        }
        Ok(reply)
    }
}

impl<R: BufRead, W: Write> Hid for Host<R, W> {
    fn write(&mut self, report: &[u8]) -> Result<usize, ModuleError> {
        Ok(self.call(Op::Write, report, 0, 0)?.len.unwrap_or(0))
    }
    fn read(&mut self, len: usize, timeout_ms: i32) -> Result<Vec<u8>, ModuleError> {
        let reply = self.call(Op::Read, &[], len, timeout_ms)?;
        decode(reply.data.as_deref().unwrap_or(""))
    }
    fn send_feature(&mut self, report: &[u8]) -> Result<usize, ModuleError> {
        Ok(self.call(Op::SendFeature, report, 0, 0)?.len.unwrap_or(0))
    }
    fn get_feature(&mut self, prefix: &[u8], len: usize) -> Result<Vec<u8>, ModuleError> {
        let reply = self.call(Op::GetFeature, prefix, len, 0)?;
        decode(reply.data.as_deref().unwrap_or(""))
    }
}

/// Serves requests until EOF or `shutdown`. On EOF the module still runs its
/// own shutdown so the device is released even if the host vanished.
pub fn serve<M: Module, R: BufRead, W: Write>(module: &mut M, input: R, output: W) {
    let mut host = Host::new(input, output);
    let mut leds = None::<usize>;
    loop {
        let line = match host.read_line() {
            Ok(Some(line)) => line,
            _ => {
                if leds.is_some() {
                    let _ = module.shutdown(None, &mut host);
                }
                return;
            }
        };
        host.calls = 0;
        let request: Request = match serde_json::from_slice(&line) {
            Ok(r) => r,
            Err(_) => return,
        };
        let outcome: Result<serde_json::Value, ModuleError> = if request.v != MODULE_API {
            Err(protocol("unsupported protocol version"))
        } else {
            match &request.method {
                Method::Hello => {
                    serde_json::to_value(module.info()).map_err(|e| protocol(e.to_string()))
                }
                Method::Identify {
                    interface,
                    platform,
                } => serde_json::to_value(
                    module.identify_with_platform(interface, platform.as_ref()),
                )
                .map_err(|e| protocol(e.to_string())),
                Method::Initialize {
                    interface,
                    zones,
                    platform,
                } => module
                    .initialize_with_platform(
                        interface,
                        zones.as_deref(),
                        platform.as_ref(),
                        &mut host,
                    )
                    .and_then(|caps| {
                        if caps.layout.leds.is_empty() || caps.layout.leds.len() > MAX_LEDS {
                            return Err(protocol("module reported an invalid layout"));
                        }
                        leds = Some(caps.layout.leds.len());
                        serde_json::to_value(caps).map_err(|e| protocol(e.to_string()))
                    }),
                Method::Frame { rgb } => match leds {
                    None => Err(protocol("frame before initialize")),
                    Some(n) => unpack_rgb(rgb, n)
                        .and_then(|colors| module.frame(&colors, &mut host))
                        .map(|_| serde_json::Value::Bool(true)),
                },
                Method::Status => {
                    serde_json::to_value(module.status()).map_err(|e| protocol(e.to_string()))
                }
                Method::Shutdown { final_color } => {
                    let result = if leds.is_some() {
                        module.shutdown(*final_color, &mut host)
                    } else {
                        Ok(())
                    };
                    leds = None;
                    result.map(|_| serde_json::Value::Bool(true))
                }
            }
        };
        let done = matches!(request.method, Method::Shutdown { .. });
        let (result, error) = match outcome {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error)),
        };
        if host
            .send_line(&Response {
                v: MODULE_API,
                id: request.id,
                result,
                error,
            })
            .is_err()
            || done
        {
            return;
        }
    }
}

/// Convenience entry point for module binaries.
pub fn run<M: Module>(mut module: M) {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(&mut module, stdin.lock(), stdout.lock());
}

#[cfg(test)]
mod platform_tests {
    use super::*;

    /// Deliberately implements no platform hooks, like existing API-1 modules.
    struct Legacy;

    impl Module for Legacy {
        fn info(&self) -> ModuleInfo {
            ModuleInfo {
                id: "legacy".into(),
                version: "1".into(),
                family: "fixture".into(),
                api: MODULE_API,
            }
        }
        fn identify(&self, _: &InterfaceInfo) -> Identification {
            Identification {
                supported: true,
                model: Some("Legacy".into()),
                kind: Some(DeviceKind::Other),
                reason: None,
            }
        }
        fn initialize(
            &mut self,
            _: &InterfaceInfo,
            _: &mut dyn Hid,
        ) -> Result<Capabilities, ModuleError> {
            Err(protocol("zone-aware default must remain in use"))
        }
        fn initialize_with_zones(
            &mut self,
            _: &InterfaceInfo,
            zones: Option<&[Zone]>,
            _: &mut dyn Hid,
        ) -> Result<Capabilities, ModuleError> {
            let zones = zones.unwrap();
            Ok(Capabilities {
                name: "Legacy".into(),
                kind: DeviceKind::Other,
                layout: freergb_core::LedLayout::strip(zones[0].leds as u16),
                firmware: None,
                restore: Restore::LastFrame,
                zones: zones.to_vec(),
            })
        }
        fn frame(&mut self, _: &[Rgb], _: &mut dyn Hid) -> Result<(), ModuleError> {
            Ok(())
        }
        fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
            Ok(())
        }
    }

    #[test]
    fn old_module_hooks_keep_working_with_both_old_and_new_hosts() {
        for platform in [
            None,
            Some(PlatformIdentity {
                board_name: Some("Fixture Board".into()),
                ..PlatformIdentity::default()
            }),
        ] {
            let interface = InterfaceInfo {
                vid: 1,
                pid: 2,
                interface: 0,
                usage_page: 1,
                usage: 2,
                product: None,
            };
            let methods = [
                Method::Identify {
                    interface: interface.clone(),
                    platform: platform.clone(),
                },
                Method::Initialize {
                    interface,
                    platform,
                    zones: Some(vec![Zone {
                        name: "Configured".into(),
                        leds: 3,
                    }]),
                },
                Method::Shutdown { final_color: None },
            ];
            let mut input = Vec::new();
            for (id, method) in methods.into_iter().enumerate() {
                serde_json::to_writer(
                    &mut input,
                    &Request {
                        v: MODULE_API,
                        id: id as u64,
                        method,
                    },
                )
                .unwrap();
                input.push(b'\n');
            }
            let mut output = Vec::new();
            serve(&mut Legacy, std::io::Cursor::new(input), &mut output);
            let replies: Vec<Response> = output
                .split(|b| *b == b'\n')
                .filter(|s| !s.is_empty())
                .map(|s| serde_json::from_slice(s).unwrap())
                .collect();
            assert_eq!(replies.len(), 3);
            assert!(replies.iter().all(|r| r.error.is_none()));
            let caps: Capabilities =
                serde_json::from_value(replies[1].result.clone().unwrap()).unwrap();
            assert_eq!(caps.layout.leds.len(), 3);
            assert_eq!(caps.zones[0].name, "Configured");
        }
    }
}
