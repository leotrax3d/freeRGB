//! Bounded API 2 child process lifecycle. No native bus implementation.
use crate::host::{Launch, Process, INITIALIZE_TIMEOUT, REQUEST_TIMEOUT};
use crate::store::VerifiedBus;
use freergb_module_api::bus::{
    Call, Info, Method, PciId, ProcessCall, Reply, Request, Response, API,
};
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, ModuleError, MAX_CALLS_PER_REQUEST, MAX_LEDS,
};
use std::time::{Duration, Instant};

/// Backend must honor the host deadline. Native blocking callbacks are not
/// supported by this implementation; use a separately bounded helper first.
pub trait Transport {
    fn capability(&self) -> u64;
    fn transact(&mut self, request: &Call, deadline: Instant) -> Reply;
    fn revoke(&mut self);
    /// Unclean process exit or protocol failure may follow completed writes.
    fn fault(&mut self) {
        self.revoke();
    }
}

fn protocol(message: &str) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

pub struct BusProcess<T: Transport> {
    process: Option<Process>,
    transport: T,
    next_id: u64,
    last_call: u64,
    leds: usize,
}

impl<T: Transport> BusProcess<T> {
    /// Policy, consent and PCI matching must be checked before constructing the
    /// transport. Package identity is compared with the child's hello response.
    pub fn start(
        verified: VerifiedBus,
        pci: PciId,
        mut transport: T,
    ) -> Result<(Self, Capabilities), ModuleError> {
        let manifest = &verified.verified.manifest;
        if !verified.policy().pci.contains(&pci) {
            transport.revoke();
            return Err(ModuleError::new(
                ErrorCode::Permission,
                "PCI identity denied",
            ));
        }
        let launch = Launch {
            module_id: manifest.id.clone(),
            executable: verified.verified.executable.clone(),
            directory: verified.verified.directory.clone(),
        };
        let process = match Process::spawn(&launch) {
            Ok(process) => process,
            Err(error) => {
                transport.revoke();
                return Err(error);
            }
        };
        let mut host = Self {
            process: Some(process),
            transport,
            next_id: 0,
            last_call: 0,
            leds: 0,
        };
        let result = (|| {
            let value = host.request(Method::Hello, REQUEST_TIMEOUT, false)?;
            let info: Info =
                serde_json::from_value(value).map_err(|_| protocol("invalid bus hello"))?;
            info.hello
                .negotiate()
                .map_err(|_| protocol("unsupported bus protocol"))?;
            if info.module.id != manifest.id
                || info.module.version != manifest.version
                || info.module.family != manifest.family
                || info.module.api != API
            {
                return Err(protocol("bus module identity mismatch"));
            }
            let identified: Identification = serde_json::from_value(host.request(
                Method::Identify { pci },
                REQUEST_TIMEOUT,
                false,
            )?)
            .map_err(|_| protocol("invalid bus identification"))?;
            if !identified.supported {
                return Err(ModuleError::new(
                    ErrorCode::Unsupported,
                    "bus device not supported",
                ));
            }
            let capability = host.transport.capability();
            let caps: Capabilities = serde_json::from_value(host.request(
                Method::Initialize { pci, capability },
                INITIALIZE_TIMEOUT,
                true,
            )?)
            .map_err(|_| protocol("invalid bus capabilities"))?;
            if caps.layout.validate().is_err()
                || caps.layout.leds.is_empty()
                || caps.layout.leds.len() > MAX_LEDS
            {
                return Err(protocol("invalid bus layout"));
            }
            host.leds = caps.layout.leds.len();
            Ok(caps)
        })();
        match result {
            Ok(caps) => Ok((host, caps)),
            Err(error) => {
                host.fault();
                Err(error)
            }
        }
    }

    fn request(
        &mut self,
        method: Method,
        timeout: Duration,
        allow_calls: bool,
    ) -> Result<serde_json::Value, ModuleError> {
        let result = self.exchange(method, timeout, allow_calls);
        if result.is_err() {
            self.fault();
        }
        result
    }

    fn exchange(
        &mut self,
        method: Method,
        timeout: Duration,
        allow_calls: bool,
    ) -> Result<serde_json::Value, ModuleError> {
        let deadline = Instant::now() + timeout;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| protocol("request counter exhausted"))?;
        let id = self.next_id;
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| protocol("bus module is closed"))?;
        process.send(&Request { v: API, id, method }, deadline)?;
        let mut calls = 0;
        loop {
            let value = process.receive(deadline)?;
            if value.get("transaction").is_some() {
                if !allow_calls {
                    return Err(protocol("bus I/O outside active device request"));
                }
                let envelope: ProcessCall =
                    serde_json::from_value(value).map_err(|_| protocol("invalid bus call"))?;
                if envelope.request != id {
                    return Err(protocol("bus call request mismatch"));
                }
                let call = envelope.transaction;
                calls += 1;
                if call.v != API || call.call <= self.last_call || calls > MAX_CALLS_PER_REQUEST {
                    return Err(protocol("invalid or excessive bus call"));
                }
                self.last_call = call.call;
                if Instant::now() >= deadline {
                    return Err(ModuleError::new(ErrorCode::Timeout, "bus request deadline"));
                }
                let reply = self.transport.transact(&call, deadline);
                let failed = reply.result.is_err();
                process.send(&reply, deadline)?;
                // A child cannot hide a broker failure by returning a successful frame.
                if failed {
                    return Err(protocol("bus transaction rejected or failed"));
                }
            } else {
                let response: Response =
                    serde_json::from_value(value).map_err(|_| protocol("invalid bus response"))?;
                if response.v != API || response.id != id {
                    return Err(protocol("bus response version/id mismatch"));
                }
                return response.result;
            }
        }
    }

    pub fn frame(&mut self, colors: &[freergb_core::Rgb]) -> Result<(), ModuleError> {
        if colors.len() != self.leds {
            return Err(protocol("bus frame length mismatch"));
        }
        self.request(
            Method::Frame {
                rgb: freergb_module_api::pack_rgb(colors),
            },
            REQUEST_TIMEOUT,
            true,
        )
        .map(|_| ())
    }

    pub fn shutdown(&mut self) -> Result<(), ModuleError> {
        let result = self
            .request(Method::Shutdown, REQUEST_TIMEOUT, true)
            .map(|_| ());
        self.close();
        result
    }

    fn close(&mut self) {
        self.transport.revoke();
        self.process = None;
    }

    fn fault(&mut self) {
        self.transport.fault();
        self.close();
    }
}

impl<T: Transport> Drop for BusProcess<T> {
    fn drop(&mut self) {
        if self.process.is_some() {
            self.fault();
        } else {
            self.close();
        }
    }
}
