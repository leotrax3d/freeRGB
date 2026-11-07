//! Bounded API 4 child process lifecycle. No native bus implementation.
use crate::host::{Launch, Process, INITIALIZE_TIMEOUT, REQUEST_TIMEOUT};
use crate::store::VerifiedTransport;
use freergb_module_api::bus::Error;
use freergb_module_api::transport::{Call, Policy, Step};
use freergb_module_api::transport::{Identity, Method, ProcessCall, Reply, Request, Response, API};
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, ModuleError, MAX_CALLS_PER_REQUEST, MAX_LEDS,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// One simulated physical bus. Clones are aliases and serialize complete recipes.
/// There is deliberately no native backend callback or OS resource constructor.
#[derive(Clone, Default)]
pub struct SimulatedBus(Arc<Shared>);

#[derive(Default)]
struct Shared {
    model: Mutex<Model>,
    queue: Mutex<Queue>,
    wake: std::sync::Condvar,
}
#[derive(Default)]
struct Queue {
    next: u64,
    tickets: std::collections::VecDeque<u64>,
}
struct Admission(SimulatedBus);
impl Drop for Admission {
    fn drop(&mut self) {
        self.0 .0.queue.lock().unwrap().tickets.pop_front();
        self.0 .0.wake.notify_all();
    }
}

#[derive(Default)]
struct Model {
    trace: Vec<Step>,
    replies: std::collections::VecDeque<Result<Vec<u8>, Error>>,
    faulted: bool,
}

impl SimulatedBus {
    pub fn trace(&self) -> Vec<Step> {
        self.0.model.lock().unwrap().trace.clone()
    }
    /// A scripted result for each operation, including empty write acknowledgments.
    pub fn script(&self, replies: impl IntoIterator<Item = Result<Vec<u8>, Error>>) {
        self.0.model.lock().unwrap().replies.extend(replies);
    }
    pub fn is_faulted(&self) -> bool {
        self.0.model.lock().unwrap().faulted
    }

    fn admit(&self, deadline: Instant) -> Result<Admission, Error> {
        let mut queue = self.0.queue.lock().map_err(|_| Error::Faulted)?;
        if queue.tickets.len() >= 32 {
            return Err(Error::Busy);
        }
        let ticket = queue.next;
        queue.next = queue.next.checked_add(1).ok_or(Error::Faulted)?;
        queue.tickets.push_back(ticket);
        loop {
            let now = Instant::now();
            if now >= deadline {
                queue.tickets.retain(|t| *t != ticket);
                self.0.wake.notify_all();
                return Err(Error::Timeout);
            }
            if queue.tickets.front() == Some(&ticket) {
                return Ok(Admission(self.clone()));
            }
            queue = self
                .0
                .wake
                .wait_timeout(queue, deadline - now)
                .map_err(|_| Error::Faulted)?
                .0;
        }
    }
}

pub struct Broker {
    policy: Policy,
    identity: Identity,
    bus: SimulatedBus,
    capability: u64,
    revoked: bool,
    last_call: u64,
}
impl Broker {
    pub fn new(policy: Policy, identity: Identity, bus: SimulatedBus) -> Result<Self, ModuleError> {
        policy
            .validate()
            .map_err(|_| protocol("invalid transport policy"))?;
        if !policy.identities.contains(&identity) {
            return Err(protocol("identity denied"));
        }
        static NEXT: Mutex<u64> = Mutex::new(1);
        let mut next = NEXT.lock().map_err(|_| protocol("capabilities faulted"))?;
        let capability = *next;
        *next = next
            .checked_add(1)
            .ok_or_else(|| protocol("capabilities exhausted"))?;
        Ok(Self {
            policy,
            identity,
            bus,
            capability,
            revoked: false,
            last_call: 0,
        })
    }
    pub fn capability(&self) -> u64 {
        self.capability
    }
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
    pub fn fault(&mut self) {
        self.revoke();
        self.bus.0.model.lock().unwrap().faulted = true;
    }
    pub fn transact(&mut self, call: &Call, deadline: Instant) -> Reply {
        let result = self.execute(call, deadline);
        if result.is_err() {
            self.revoke();
        }
        Reply {
            v: API,
            call: call.call,
            result,
        }
    }
    fn execute(&mut self, call: &Call, deadline: Instant) -> Result<Vec<Vec<u8>>, Error> {
        if self.revoked
            || call.v != API
            || call.capability != self.capability
            || call.call <= self.last_call
        {
            return Err(Error::Denied);
        }
        self.last_call = call.call;
        self.policy
            .authorize(&self.identity, &call.recipe, &call.steps)?;
        let _admission = self.bus.admit(deadline)?;
        // All policy checks precede dispatch. Admission spans the entire recipe.
        // try_lock plus deadline bounds waiting; simulator operations never block.
        let mut model = loop {
            if Instant::now() >= deadline {
                return Err(Error::Timeout);
            }
            match self.bus.0.model.try_lock() {
                Ok(model) => break model,
                Err(std::sync::TryLockError::WouldBlock) => std::thread::yield_now(),
                Err(_) => return Err(Error::Faulted),
            }
        };
        if model.faulted {
            return Err(Error::Faulted);
        }
        let mut replies = Vec::new();
        for step in &call.steps {
            if Instant::now() >= deadline {
                model.faulted = true;
                return Err(Error::Timeout);
            }
            model.trace.push(step.clone());
            let len = if step.operation.reads() {
                step.len()
            } else {
                0
            };
            let reply = model
                .replies
                .pop_front()
                .unwrap_or_else(|| Ok(vec![0; len]));
            match reply {
                Ok(bytes) if bytes.len() == len => replies.push(bytes),
                Ok(_) => {
                    model.faulted = true;
                    return Err(Error::Faulted);
                }
                Err(error) => {
                    model.faulted = true;
                    return Err(error);
                }
            }
        }
        Ok(replies)
    }
}

fn protocol(message: &str) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

pub struct TransportProcess {
    process: Option<Process>,
    transport: Broker,
    next_id: u64,
    last_call: u64,
    leds: usize,
}

impl TransportProcess {
    /// Policy, consent and PCI matching must be checked before constructing the
    /// transport. Package identity is compared with the child's hello response.
    pub fn start(
        verified: VerifiedTransport,
        identity: Identity,
        bus: SimulatedBus,
        consent: bool,
    ) -> Result<(Self, Capabilities), ModuleError> {
        let manifest = &verified.verified.manifest;
        if !consent || !verified.policy().identities.contains(&identity) {
            return Err(ModuleError::new(
                ErrorCode::Permission,
                "transport identity or consent denied",
            ));
        }
        let mut transport = Broker::new(verified.policy().clone(), identity.clone(), bus)?;
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
            let info: freergb_module_api::ModuleInfo =
                serde_json::from_value(value).map_err(|_| protocol("invalid bus hello"))?;
            if info.id != manifest.id
                || info.version != manifest.version
                || info.family != manifest.family
                || info.api != API
            {
                return Err(protocol("bus module identity mismatch"));
            }
            let identified: Identification = serde_json::from_value(host.request(
                Method::Identify {
                    identity: identity.clone(),
                },
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
                Method::Initialize {
                    identity,
                    capability,
                },
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

impl Drop for TransportProcess {
    fn drop(&mut self) {
        if self.process.is_some() {
            self.fault();
        } else {
            self.close();
        }
    }
}
