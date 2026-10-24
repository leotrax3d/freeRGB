//! Simulation-only API v2 host. No native backend, hardware enumeration or I/O.
//! An instance represents one canonical simulated physical bus; sessions share
//! its FIFO queue. Production module setup continues to accept HID API v1 only.
use freergb_module_api::bus::{
    Call, Error, Hello, PciId, Policy, Reply, Transfer, API, MAX_OPERATIONS,
};
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const MAX_QUEUE: usize = 64;
static NEXT_CAPABILITY: Mutex<u64> = Mutex::new(1);

/// A register bus with wire traces. All storage is memory owned by the caller.
#[derive(Default)]
pub struct RegisterBus {
    devices: BTreeMap<u8, [u8; 256]>,
    trace: Vec<Vec<u8>>,
    failure: Option<(usize, Error)>,
}

impl RegisterBus {
    pub fn attach(&mut self, address: u8, registers: [u8; 256]) {
        self.devices.insert(address, registers);
    }

    /// Inject a one-shot failure before the next operation. No retry occurs.
    pub fn fail_next(&mut self, error: Error) {
        self.fail_after(0, error);
    }

    pub fn fail_after(&mut self, completed: usize, error: Error) {
        self.failure = Some((completed, error));
    }

    pub fn trace(&self) -> &[Vec<u8>] {
        &self.trace
    }

    fn transfer(&mut self, request: &Transfer) -> Result<Vec<u8>, Error> {
        use freergb_module_api::bus::Operation;
        if let Some((remaining, error)) = self.failure {
            if remaining == 0 {
                self.failure = None;
                return Err(error);
            }
            self.failure = Some((remaining - 1, error));
        }
        let registers = self.devices.get_mut(&request.address).ok_or(Error::Nack)?;
        let start = usize::from(request.register);
        let end = start
            .checked_add(request.len())
            .filter(|end| *end <= 256)
            .ok_or(Error::Denied)?;
        let mut wire = vec![request.address << 1, request.register];
        if request.op.writes() {
            if request.op == Operation::SmbusWriteBlockData {
                wire.push(request.data.len() as u8);
            }
            wire.extend_from_slice(&request.data);
            registers[start..end].copy_from_slice(&request.data);
            self.trace.push(wire);
            Ok(vec![])
        } else {
            wire.push((request.address << 1) | 1);
            if request.op == Operation::SmbusReadBlockData {
                wire.push(request.read_len);
            }
            let data = registers[start..end].to_vec();
            wire.extend_from_slice(&data);
            self.trace.push(wire);
            Ok(data)
        }
    }
}

struct Admission {
    next: u64,
    queue: VecDeque<u64>,
    active: bool,
    faulted: bool,
}

pub struct SimulatedBus {
    admission: Mutex<Admission>,
    changed: Condvar,
    registers: Mutex<RegisterBus>,
}

impl SimulatedBus {
    /// Explicit simulation route: verifies installed files before any process
    /// starts, and derives policy only from that verified package.
    pub fn start_module(
        self: &Arc<Self>,
        store: &freergb_modules::store::Store,
        id: &str,
        pci: PciId,
        consent: bool,
    ) -> Result<
        (
            freergb_modules::bus_host::BusProcess<Session>,
            freergb_module_api::Capabilities,
        ),
        freergb_module_api::ModuleError,
    > {
        use freergb_module_api::{ErrorCode, ModuleError};
        let verified = store.verified_bus(id).map_err(|_| {
            ModuleError::new(ErrorCode::Permission, "bus package verification failed")
        })?;
        let hello = Hello {
            v: API,
            api: vec![API],
            transport: "simulation".into(),
        };
        let session = self
            .session(&hello, verified.policy().clone(), pci, consent)
            .map_err(|_| ModuleError::new(ErrorCode::Permission, "bus session denied"))?;
        freergb_modules::bus_host::BusProcess::start(verified, pci, session)
    }
    pub fn new(registers: RegisterBus) -> Arc<Self> {
        Arc::new(Self {
            admission: Mutex::new(Admission {
                next: 0,
                queue: VecDeque::new(),
                active: false,
                faulted: false,
            }),
            changed: Condvar::new(),
            registers: Mutex::new(registers),
        })
    }

    /// Consent and policy are host inputs, never fields supplied by a bus call.
    /// Caller must use policy from a verified schema-2 package, not child output.
    pub fn session(
        self: &Arc<Self>,
        hello: &Hello,
        policy: Policy,
        pci: PciId,
        consent: bool,
    ) -> Result<Session, Error> {
        hello.negotiate()?;
        policy.validate()?;
        if !consent || !policy.pci.contains(&pci) {
            return Err(Error::Denied);
        }
        let mut next = NEXT_CAPABILITY.lock().map_err(|_| Error::Faulted)?;
        let capability = *next;
        *next = next.checked_add(1).ok_or(Error::Faulted)?;
        Ok(Session {
            bus: self.clone(),
            policy,
            capability,
            revoked: AtomicBool::new(false),
        })
    }

    pub fn trace(&self) -> Vec<Vec<u8>> {
        self.registers.lock().unwrap().trace.clone()
    }

    fn admit(
        &self,
        deadline: Instant,
        cancelled: &AtomicBool,
        revoked: &AtomicBool,
    ) -> Result<Guard<'_>, Error> {
        let mut state = self.admission.lock().map_err(|_| Error::Faulted)?;
        if state.queue.len() >= MAX_QUEUE {
            return Err(Error::Busy);
        }
        let ticket = state.next;
        state.next = state.next.checked_add(1).ok_or(Error::Faulted)?;
        state.queue.push_back(ticket);
        loop {
            let error = if state.faulted {
                Some(Error::Faulted)
            } else if cancelled.load(Ordering::Acquire) || revoked.load(Ordering::Acquire) {
                Some(Error::Cancelled)
            } else if Instant::now() >= deadline {
                Some(Error::Timeout)
            } else {
                None
            };
            if let Some(error) = error {
                state.queue.retain(|id| *id != ticket);
                self.changed.notify_all();
                return Err(error);
            }
            if !state.active && state.queue.front() == Some(&ticket) {
                state.queue.pop_front();
                state.active = true;
                return Ok(Guard(self));
            }
            let wait = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(10));
            state = self
                .changed
                .wait_timeout(state, wait)
                .map_err(|_| Error::Faulted)?
                .0;
        }
    }
}

struct Guard<'a>(&'a SimulatedBus);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        let mut state = self.0.admission.lock().unwrap_or_else(|e| e.into_inner());
        state.active = false;
        // An unexpected unwind might have interrupted a mutation.
        state.faulted |= std::thread::panicking();
        self.0.changed.notify_all();
    }
}

pub struct Session {
    bus: Arc<SimulatedBus>,
    policy: Policy,
    capability: u64,
    revoked: AtomicBool,
}

impl Session {
    pub fn capability(&self) -> u64 {
        self.capability
    }
    pub fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
        self.bus.changed.notify_all();
    }

    /// Bounded wire dispatch for fixtures. Framing/process lifetime is the
    /// caller's responsibility; this never starts a module or opens hardware.
    pub fn call_json(&self, bytes: &[u8], cancelled: &AtomicBool) -> Result<Reply, Error> {
        if bytes.len() > freergb_module_api::MAX_MESSAGE {
            return Err(Error::Denied);
        }
        let call: Call = serde_json::from_slice(bytes).map_err(|_| Error::Denied)?;
        Ok(self.call(&call, cancelled))
    }

    pub fn call(&self, request: &Call, cancelled: &AtomicBool) -> Reply {
        self.call_until(request, cancelled, Instant::now() + Duration::from_secs(2))
    }

    fn call_until(&self, request: &Call, cancelled: &AtomicBool, deadline: Instant) -> Reply {
        let reply = Reply {
            v: API,
            call: request.call,
            result: self.execute(request, cancelled, deadline),
        };
        tracing::debug!(call = request.call, operations = request.transfers.len(),
            outcome = ?reply.result.as_ref().map(|_| ()).map_err(|e| *e),
            "simulated bus transaction");
        reply
    }

    fn execute(
        &self,
        request: &Call,
        cancelled: &AtomicBool,
        deadline: Instant,
    ) -> Result<Vec<Vec<u8>>, Error> {
        if request.v != API {
            return Err(Error::Unsupported);
        }
        if request.capability != self.capability || self.revoked.load(Ordering::Acquire) {
            return Err(Error::Denied);
        }
        if request.transfers.is_empty() || request.transfers.len() > MAX_OPERATIONS {
            return Err(Error::Denied);
        }
        // Validate the entire batch before dispatching any byte.
        for transfer in &request.transfers {
            self.policy.authorize(transfer)?;
        }
        let deadline = deadline.min(Instant::now() + Duration::from_secs(2));
        let _guard = self.bus.admit(deadline, cancelled, &self.revoked)?;
        let mut registers = self.bus.registers.lock().map_err(|_| Error::Faulted)?;
        let mut replies = Vec::new();
        for transfer in &request.transfers {
            let result =
                if cancelled.load(Ordering::Acquire) || self.revoked.load(Ordering::Acquire) {
                    Err(Error::Cancelled)
                } else if Instant::now() >= deadline {
                    Err(Error::Timeout)
                } else {
                    registers.transfer(transfer)
                };
            match result {
                Ok(data) => replies.push(data),
                Err(error) => {
                    // No speculative restore or retry; partial completion is unknown.
                    self.revoke();
                    self.bus
                        .admission
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .faulted = true;
                    return Err(error);
                }
            }
        }
        Ok(replies)
    }
}

impl freergb_modules::bus_host::Transport for Session {
    fn capability(&self) -> u64 {
        self.capability()
    }
    fn transact(&mut self, request: &Call, deadline: Instant) -> Reply {
        self.call_until(request, &AtomicBool::new(false), deadline)
    }
    fn revoke(&mut self) {
        Session::revoke(self);
    }
}

#[cfg(test)]
mod tests;
