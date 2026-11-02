//! Module-side API v2 process support. No native transport or device paths.
use super::{
    Call, Error, Hello, Info, Method, PciId, ProcessCall, Reply, Request, Response, Transfer, API,
    MAX_OPERATIONS,
};
use crate::{
    Capabilities, ErrorCode, Identification, ModuleError, ModuleInfo, MAX_CALLS_PER_REQUEST,
    MAX_LEDS, MAX_MESSAGE,
};
use freergb_core::Rgb;
use std::io::{BufRead, Read, Write};

pub trait Bus {
    fn transaction(&mut self, transfers: Vec<Transfer>) -> Result<Vec<Vec<u8>>, Error>;
}

pub trait Module {
    fn info(&self) -> ModuleInfo;
    fn identify(&self, pci: PciId) -> Identification;
    fn initialize(&mut self, pci: PciId, bus: &mut dyn Bus) -> Result<Capabilities, ModuleError>;
    fn frame(&mut self, colors: &[Rgb], bus: &mut dyn Bus) -> Result<(), ModuleError>;
    fn shutdown(&mut self, bus: &mut dyn Bus) -> Result<(), ModuleError>;
}

fn protocol(message: &str) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

fn read_line(input: &mut impl BufRead) -> Result<Option<Vec<u8>>, ModuleError> {
    let mut line = Vec::new();
    let count = input
        .take((MAX_MESSAGE + 1) as u64)
        .read_until(b'\n', &mut line)
        .map_err(|_| protocol("cannot read host message"))?;
    if count == 0 {
        return Ok(None);
    }
    if line.len() > MAX_MESSAGE || line.last() != Some(&b'\n') {
        return Err(protocol("oversized or incomplete host message"));
    }
    Ok(Some(line))
}

fn send(output: &mut impl Write, message: &impl serde::Serialize) -> Result<(), ModuleError> {
    let mut bytes = serde_json::to_vec(message).map_err(|_| protocol("cannot encode message"))?;
    if bytes.len() >= MAX_MESSAGE {
        return Err(protocol("message too large"));
    }
    bytes.push(b'\n');
    output
        .write_all(&bytes)
        .and_then(|_| output.flush())
        .map_err(|_| protocol("host pipe closed"))
}

struct Host<'a, R, W> {
    input: &'a mut R,
    output: &'a mut W,
    capability: u64,
    next_call: &'a mut u64,
    request: u64,
    calls: u32,
}

impl<R: BufRead, W: Write> Bus for Host<'_, R, W> {
    fn transaction(&mut self, transfers: Vec<Transfer>) -> Result<Vec<Vec<u8>>, Error> {
        if transfers.is_empty()
            || transfers.len() > MAX_OPERATIONS
            || self.calls >= MAX_CALLS_PER_REQUEST
        {
            return Err(Error::Denied);
        }
        self.calls += 1;
        *self.next_call = self.next_call.checked_add(1).ok_or(Error::Faulted)?;
        let call = Call {
            v: API,
            call: *self.next_call,
            capability: self.capability,
            transfers,
        };
        send(
            self.output,
            &ProcessCall {
                request: self.request,
                transaction: call.clone(),
            },
        )
        .map_err(|_| Error::Disconnected)?;
        let line = read_line(self.input)
            .map_err(|_| Error::Faulted)?
            .ok_or(Error::Disconnected)?;
        let reply: Reply = serde_json::from_slice(&line).map_err(|_| Error::Faulted)?;
        if reply.v != API || reply.call != call.call {
            return Err(Error::Faulted);
        }
        let bytes = reply.result?;
        if bytes.len() != call.transfers.len()
            || bytes.iter().zip(&call.transfers).any(|(bytes, transfer)| {
                bytes.len()
                    != if transfer.op.writes() {
                        0
                    } else {
                        transfer.len()
                    }
            })
        {
            return Err(Error::Faulted);
        }
        Ok(bytes)
    }
}

pub fn serve(module: &mut impl Module, mut input: impl BufRead, mut output: impl Write) {
    let mut negotiated = false;
    let mut identified = None;
    let mut leds = None;
    let mut capability = None;
    let mut next_call = 0;
    let mut previous_id = 0;
    loop {
        let line = match read_line(&mut input) {
            Ok(Some(line)) => line,
            _ => return,
        };
        let request: Request = match serde_json::from_slice(&line) {
            Ok(request) => request,
            Err(_) => return,
        };
        if request.v != API || request.id <= previous_id {
            return;
        }
        previous_id = request.id;
        let done = matches!(request.method, Method::Shutdown);
        let result = match request.method {
            Method::Hello if !negotiated => {
                let info = module.info();
                if info.api != API {
                    Err(protocol("module does not support API 2"))
                } else {
                    negotiated = true;
                    serde_json::to_value(Info {
                        module: info,
                        hello: Hello {
                            v: API,
                            api: vec![API],
                            transport: "simulation".into(),
                        },
                    })
                    .map_err(|_| protocol("cannot encode hello"))
                }
            }
            Method::Identify { pci } if negotiated && capability.is_none() => {
                let result = module.identify(pci);
                identified = result.supported.then_some(pci);
                serde_json::to_value(result).map_err(|_| protocol("cannot encode identification"))
            }
            Method::Initialize {
                pci,
                capability: assigned,
            } if identified == Some(pci) && capability.is_none() => {
                capability = Some(assigned);
                let mut bus = Host {
                    input: &mut input,
                    output: &mut output,
                    capability: assigned,
                    next_call: &mut next_call,
                    request: request.id,
                    calls: 0,
                };
                module.initialize(pci, &mut bus).and_then(|caps| {
                    if caps.layout.validate().is_err()
                        || caps.layout.leds.is_empty()
                        || caps.layout.leds.len() > MAX_LEDS
                    {
                        return Err(protocol("invalid layout"));
                    }
                    leds = Some(caps.layout.leds.len());
                    serde_json::to_value(caps).map_err(|_| protocol("cannot encode capabilities"))
                })
            }
            Method::Frame { rgb } if leds.is_some() => {
                let mut bus = Host {
                    input: &mut input,
                    output: &mut output,
                    capability: capability.unwrap(),
                    next_call: &mut next_call,
                    request: request.id,
                    calls: 0,
                };
                crate::unpack_rgb(&rgb, leds.unwrap())
                    .and_then(|colors| module.frame(&colors, &mut bus))
                    .map(|_| serde_json::Value::Bool(true))
            }
            Method::Shutdown if negotiated => {
                if let Some(assigned) = capability {
                    let mut bus = Host {
                        input: &mut input,
                        output: &mut output,
                        capability: assigned,
                        next_call: &mut next_call,
                        request: request.id,
                        calls: 0,
                    };
                    module
                        .shutdown(&mut bus)
                        .map(|_| serde_json::Value::Bool(true))
                } else {
                    Ok(serde_json::Value::Bool(true))
                }
            }
            _ => Err(protocol("invalid bus module lifecycle")),
        };
        let failed = result.is_err();
        if send(
            &mut output,
            &Response {
                v: API,
                id: request.id,
                result,
            },
        )
        .is_err()
            || done
            || failed
        {
            return;
        }
    }
}

pub fn run(mut module: impl Module) {
    serve(
        &mut module,
        std::io::stdin().lock(),
        std::io::stdout().lock(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn calls_are_bound_to_parent_request_and_replies_are_strict() {
        for (reply, expected) in [
            (
                Reply {
                    v: API,
                    call: 1,
                    result: Ok(vec![vec![0xab]]),
                },
                Ok(vec![vec![0xab]]),
            ),
            (
                Reply {
                    v: 1,
                    call: 1,
                    result: Ok(vec![vec![0xab]]),
                },
                Err(Error::Faulted),
            ),
            (
                Reply {
                    v: API,
                    call: 2,
                    result: Ok(vec![vec![0xab]]),
                },
                Err(Error::Faulted),
            ),
            (
                Reply {
                    v: API,
                    call: 1,
                    result: Ok(vec![vec![]]),
                },
                Err(Error::Faulted),
            ),
            (
                Reply {
                    v: API,
                    call: 1,
                    result: Ok(vec![]),
                },
                Err(Error::Faulted),
            ),
            (
                Reply {
                    v: API,
                    call: 1,
                    result: Err(Error::Denied),
                },
                Err(Error::Denied),
            ),
        ] {
            let mut bytes = serde_json::to_vec(&reply).unwrap();
            bytes.push(b'\n');
            let mut input = Cursor::new(bytes);
            let mut output = vec![];
            let mut next = 0;
            let mut host = Host {
                input: &mut input,
                output: &mut output,
                capability: 9,
                next_call: &mut next,
                request: 3,
                calls: 0,
            };
            assert_eq!(
                host.transaction(vec![Transfer {
                    op: super::super::Operation::SmbusReadByteData,
                    address: 0x18,
                    register: 0,
                    data: vec![],
                    read_len: 1
                }]),
                expected
            );
            let sent: ProcessCall = serde_json::from_slice(&output).unwrap();
            assert_eq!(sent.request, 3);
            assert_eq!(sent.transaction.capability, 9);
            assert_eq!(sent.transaction.call, 1);
        }
    }

    #[test]
    fn sdk_limits_unterminated_input_and_excess_calls() {
        assert!(read_line(&mut Cursor::new(vec![b' '; MAX_MESSAGE + 1])).is_err());
        let mut next = 0;
        let mut input = Cursor::new(Vec::<u8>::new());
        let mut output = vec![];
        let mut host = Host {
            input: &mut input,
            output: &mut output,
            capability: 1,
            next_call: &mut next,
            request: 1,
            calls: MAX_CALLS_PER_REQUEST,
        };
        assert_eq!(
            host.transaction(vec![Transfer {
                op: super::super::Operation::SmbusReadByteData,
                address: 0x18,
                register: 0,
                data: vec![],
                read_len: 1
            }]),
            Err(Error::Denied)
        );
        assert!(output.is_empty());
    }
}
