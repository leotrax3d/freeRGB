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
    calls: u32,
}

fn protocol(message: impl Into<String>) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

impl<R: BufRead, W: Write> Host<R, W> {
    pub fn new(input: R, output: W) -> Self {
        Self {
            input,
            output,
            next_call: 0,
            calls: 0,
        }
    }

    fn send_line(&mut self, value: &impl Serialize) -> Result<(), ModuleError> {
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
    fn read_line(&mut self) -> Result<Option<Vec<u8>>, ModuleError> {
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
            v: MODULE_API,
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
        if reply.v != MODULE_API || reply.call != call.call {
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
                Method::Identify { interface } => serde_json::to_value(module.identify(interface))
                    .map_err(|e| protocol(e.to_string())),
                Method::Initialize { interface, zones } => module
                    .initialize_with_zones(interface, zones.as_deref(), &mut host)
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
