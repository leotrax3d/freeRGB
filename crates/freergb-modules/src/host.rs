//! Runs one module process per device and brokers its transport calls.
//!
//! Handle ownership: the daemon opens the HID interface and keeps it for the
//! device's whole lifetime; the child only sees report bytes. Every report
//! still passes the daemon's process-wide HID gate, so built-in adapters and
//! modules never touch the bus concurrently. A crashed or hung child is
//! killed and restarted by the engine's back-off; after
//! [`MAX_CONSECUTIVE_FAILURES`] the device stays faulted until set up again.
use freergb_core::{Frame, LedLayout, Rgb};
use freergb_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, HidTransport,
    RgbDevice, TransportError,
};
use freergb_module_api::{
    decode, encode, pack_rgb, Call, CallReply, Capabilities, ErrorCode, InterfaceInfo, Method,
    ModuleError, ModuleInfo, Op, PlatformIdentity, Request, Response, Zone, MAX_CALLS_PER_REQUEST,
    MAX_MESSAGE, MAX_REPORT, MODULE_API,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::time::{Duration, Instant};

pub const MAX_CONSECUTIVE_FAILURES: u32 = 5;
/// Deadline for one request including all of its transport calls.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
pub const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(10);

/// How the child is started.
#[derive(Debug, Clone)]
pub struct Launch {
    pub module_id: String,
    pub executable: PathBuf,
    pub directory: PathBuf,
}

fn protocol(message: impl Into<String>) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, message)
}

fn tick_interval(value: &serde_json::Value) -> Result<Option<Duration>, ModuleError> {
    Ok(match value.get("timing") {
        None => None,
        Some(timing) => {
            if timing.get("v").and_then(|v| v.as_u64()) != Some(1) {
                return Err(protocol("unsupported timing extension"));
            }
            match timing.get("tick_ms") {
                None | Some(serde_json::Value::Null) => None,
                Some(ms) => match ms.as_u64() {
                    Some(ms)
                        if (freergb_module_api::MIN_TICK_MS..=freergb_module_api::MAX_TICK_MS)
                            .contains(&ms) =>
                    {
                        Some(Duration::from_millis(ms))
                    }
                    _ => return Err(protocol("invalid tick interval")),
                },
            }
        }
    })
}

fn sequence_delay(
    call: &Call,
    total_ms: &mut u64,
    deadline: Instant,
) -> Result<Duration, ModuleError> {
    let ms = u64::try_from(call.timeout_ms).unwrap_or(u64::MAX);
    *total_ms = total_ms.saturating_add(ms);
    if ms == 0
        || ms > freergb_module_api::MAX_DELAY_MS
        || *total_ms > freergb_module_api::MAX_SEQUENCE_DELAY_MS
        || !call.data.is_empty()
        || call.len != 0
    {
        return Err(protocol("invalid or excessive sequence delay"));
    }
    let duration = Duration::from_millis(ms);
    if Instant::now() + duration >= deadline {
        return Err(ModuleError::new(
            ErrorCode::Timeout,
            "sequence delay exceeds request deadline",
        ));
    }
    Ok(duration)
}

/// One running child process. Lines are read by a helper thread into a
/// bounded channel so a deadline can be enforced on every read; writes go
/// through a second thread so a child that stops reading cannot block us.
pub struct Process {
    child: Child,
    lines: Receiver<std::io::Result<Vec<u8>>>,
    writes: Option<SyncSender<Vec<u8>>>,
    next_id: u64,
}

impl Process {
    pub fn spawn(launch: &Launch) -> Result<Self, ModuleError> {
        let mut command = Command::new(&launch.executable);
        command
            .current_dir(&launch.directory)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // Windows processes need SystemRoot for basic DLL loading.
        for keep in ["SystemRoot", "SYSTEMROOT", "windir"] {
            if let Some(value) = std::env::var_os(keep) {
                command.env(keep, value);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn().map_err(|e| {
            ModuleError::new(ErrorCode::Internal, format!("cannot start module: {e}"))
        })?;
        let stdout = child.stdout.take().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let (line_tx, lines) = sync_channel(4);
        std::thread::Builder::new()
            .name(format!("module-{}-out", launch.module_id))
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                loop {
                    let mut line = Vec::new();
                    let result = (&mut reader)
                        .take((MAX_MESSAGE + 1) as u64)
                        .read_until(b'\n', &mut line)
                        .map(|_| line);
                    let stop = !matches!(&result, Ok(l) if l.last() == Some(&b'\n'));
                    if line_tx.send(result).is_err() || stop {
                        return;
                    }
                }
            })
            .map_err(|e| ModuleError::new(ErrorCode::Internal, e.to_string()))?;
        let (write_tx, write_rx) = sync_channel::<Vec<u8>>(4);
        std::thread::Builder::new()
            .name(format!("module-{}-in", launch.module_id))
            .spawn(move || {
                for message in write_rx {
                    if stdin
                        .write_all(&message)
                        .and_then(|_| stdin.flush())
                        .is_err()
                    {
                        return;
                    }
                }
            })
            .map_err(|e| ModuleError::new(ErrorCode::Internal, e.to_string()))?;
        Ok(Self {
            child,
            lines,
            writes: Some(write_tx),
            next_id: 0,
        })
    }

    pub(crate) fn send(
        &mut self,
        value: &impl serde::Serialize,
        deadline: Instant,
    ) -> Result<(), ModuleError> {
        let mut line = serde_json::to_vec(value).map_err(|e| protocol(e.to_string()))?;
        if line.len() >= MAX_MESSAGE {
            return Err(protocol("message too large"));
        }
        line.push(b'\n');
        let writes = self
            .writes
            .as_ref()
            .ok_or_else(|| protocol("module input closed"))?;
        // Bounded queue: a full queue means the child stopped reading.
        loop {
            match writes.try_send(line) {
                Ok(()) => return Ok(()),
                Err(std::sync::mpsc::TrySendError::Full(back)) => {
                    if Instant::now() >= deadline {
                        return Err(ModuleError::new(
                            ErrorCode::Timeout,
                            "module stopped reading",
                        ));
                    }
                    line = back;
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(_) => return Err(ModuleError::new(ErrorCode::Disconnected, "module exited")),
            }
        }
    }

    pub(crate) fn receive(&mut self, deadline: Instant) -> Result<serde_json::Value, ModuleError> {
        let wait = deadline.saturating_duration_since(Instant::now());
        match self.lines.recv_timeout(wait) {
            Ok(Ok(line)) if line.last() == Some(&b'\n') && line.len() <= MAX_MESSAGE => {
                serde_json::from_slice(&line)
                    .map_err(|e| protocol(format!("invalid JSON from module: {e}")))
            }
            Ok(Ok(line)) if line.is_empty() => {
                Err(ModuleError::new(ErrorCode::Disconnected, "module exited"))
            }
            Ok(Ok(_)) => Err(protocol("oversized or incomplete module message")),
            Ok(Err(e)) => Err(ModuleError::new(ErrorCode::Disconnected, e.to_string())),
            Err(RecvTimeoutError::Timeout) => Err(ModuleError::new(
                ErrorCode::Timeout,
                "module did not answer in time",
            )),
            Err(RecvTimeoutError::Disconnected) => {
                Err(ModuleError::new(ErrorCode::Disconnected, "module exited"))
            }
        }
    }

    /// Sends one request and serves transport calls until its answer.
    pub fn request(
        &mut self,
        method: Method,
        timeout: Duration,
        transport: &mut Option<Box<dyn HidTransport>>,
    ) -> Result<serde_json::Value, ModuleError> {
        self.request_inner(method, timeout, transport, None)
    }

    /// Runs the optional identity extension against a signed package policy.
    pub fn probe(
        &mut self,
        interface: InterfaceInfo,
        policy: &freergb_module_api::probe::Policy,
        transport: &mut Option<Box<dyn HidTransport>>,
    ) -> Result<freergb_module_api::Identification, ModuleError> {
        policy.validate()?;
        let value = self.request_inner(
            Method::Probe {
                version: freergb_module_api::probe::VERSION,
                interface,
            },
            REQUEST_TIMEOUT,
            transport,
            Some(policy),
        )?;
        serde_json::from_value(value).map_err(|_| protocol("invalid probe result"))
    }

    fn request_inner(
        &mut self,
        method: Method,
        timeout: Duration,
        transport: &mut Option<Box<dyn HidTransport>>,
        policy: Option<&freergb_module_api::probe::Policy>,
    ) -> Result<serde_json::Value, ModuleError> {
        let probing = matches!(method, Method::Probe { .. });
        if probing && policy.is_none() {
            return Err(protocol("probe requires an explicit policy"));
        }
        let deadline = Instant::now() + timeout;
        self.next_id += 1;
        let id = self.next_id;
        self.send(
            &Request {
                v: MODULE_API,
                id,
                method,
            },
            deadline,
        )?;
        let mut calls = 0u32;
        let mut delay_ms = 0u64;
        loop {
            if Instant::now() >= deadline {
                return Err(ModuleError::new(
                    ErrorCode::Timeout,
                    "module request deadline expired",
                ));
            }
            let message = self.receive(deadline)?;
            if message.get("call").is_some() {
                let call: Call =
                    serde_json::from_value(message).map_err(|e| protocol(e.to_string()))?;
                calls += 1;
                if call.v != MODULE_API || calls > MAX_CALLS_PER_REQUEST {
                    return Err(protocol("invalid or excessive transport call"));
                }
                let reply = if probing {
                    broker_probe(&call, policy.unwrap(), transport)
                } else if matches!(call.op, Op::SerialString | Op::ProductString) {
                    denied(&call, "identity reads are only allowed during probe")
                } else if call.op == Op::Delay {
                    if transport.is_none() {
                        return Err(protocol("delay without assigned device"));
                    }
                    std::thread::sleep(sequence_delay(&call, &mut delay_ms, deadline)?);
                    CallReply {
                        v: MODULE_API,
                        call: call.call,
                        data: None,
                        len: Some(0),
                        error: None,
                    }
                } else {
                    broker(&call, transport)
                };
                self.send(&reply, deadline)?;
                continue;
            }
            let response: Response =
                serde_json::from_value(message).map_err(|e| protocol(e.to_string()))?;
            if response.v != MODULE_API || response.id != id {
                return Err(protocol("module reply version/id mismatch"));
            }
            if let Some(error) = response.error {
                return Err(error);
            }
            return response
                .result
                .ok_or_else(|| protocol("module reply has no result"));
        }
    }

    pub fn hello(&mut self) -> Result<ModuleInfo, ModuleError> {
        let value = self.request(Method::Hello, REQUEST_TIMEOUT, &mut None)?;
        let info: ModuleInfo =
            serde_json::from_value(value).map_err(|e| protocol(e.to_string()))?;
        if info.api != MODULE_API {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                "module speaks another protocol version",
            ));
        }
        Ok(info)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // Closing stdin is the module's EOF signal (it then releases the
        // device itself); kill afterwards bounds the wait.
        self.writes = None;
        let until = Instant::now() + Duration::from_millis(500);
        while Instant::now() < until {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn transport_error(e: TransportError) -> String {
    match e {
        TransportError::Disconnected => "device disconnected".into(),
        TransportError::Timeout => "timed out".into(),
        other => other.to_string(),
    }
}

/// Executes one call on the daemon-owned handle.
pub(crate) fn broker(call: &Call, transport: &mut Option<Box<dyn HidTransport>>) -> CallReply {
    let mut reply = CallReply {
        v: MODULE_API,
        call: call.call,
        data: None,
        len: None,
        error: None,
    };
    let Some(transport) = transport.as_mut() else {
        reply.error = Some("no device assigned for this request".into());
        return reply;
    };
    let data = match decode(&call.data) {
        Ok(d) if d.len() <= MAX_REPORT && call.len <= MAX_REPORT => d,
        _ => {
            reply.error = Some("invalid or oversized report".into());
            return reply;
        }
    };
    let result = match call.op {
        Op::Delay => {
            reply.error = Some("delay requires a bounded request context".into());
            return reply;
        }
        Op::Write => transport.write(&data).map(|n| (None, n)),
        Op::SendFeature => transport.send_feature(&data).map(|n| (None, n)),
        Op::Read => {
            let mut buffer = vec![0u8; call.len.max(1)];
            transport
                .read_timeout(&mut buffer, call.timeout_ms.clamp(0, 1000))
                .and_then(|n| {
                    buffer
                        .get(..n)
                        .map(|bytes| (Some(bytes.to_vec()), n))
                        .ok_or_else(|| TransportError::Io("invalid transport reply length".into()))
                })
        }
        Op::SerialString | Op::ProductString => {
            return denied(call, "identity reads are only allowed during probe");
        }
        Op::GetFeature => {
            let mut buffer = vec![0u8; call.len.max(1)];
            let prefix = data.len().min(buffer.len());
            buffer[..prefix].copy_from_slice(&data[..prefix]);
            transport.get_feature(&mut buffer).and_then(|n| {
                buffer
                    .get(..n)
                    .map(|bytes| (Some(bytes.to_vec()), n))
                    .ok_or_else(|| TransportError::Io("invalid transport reply length".into()))
            })
        }
    };
    match result {
        Ok((bytes, n)) => {
            reply.data = bytes.map(|b| encode(&b));
            reply.len = Some(n);
        }
        Err(e) => reply.error = Some(transport_error(e)),
    }
    reply
}

fn denied(call: &Call, reason: &str) -> CallReply {
    CallReply {
        v: MODULE_API,
        call: call.call,
        data: None,
        len: None,
        error: Some(reason.into()),
    }
}

/// This boundary is enforced even for modules bypassing the SDK.
fn broker_probe(
    call: &Call,
    policy: &freergb_module_api::probe::Policy,
    transport: &mut Option<Box<dyn HidTransport>>,
) -> CallReply {
    if policy.validate().is_err() || call.v != MODULE_API || call.timeout_ms != 0 {
        return denied(call, "invalid probe policy or call");
    }
    let Ok(data) = decode(&call.data) else {
        return denied(call, "invalid probe data");
    };
    match call.op {
        Op::GetFeature
            if data.len() == 1
                && call.len > 0
                && policy
                    .features
                    .iter()
                    .any(|f| f.report_id == data[0] && call.len <= f.max_len) =>
        {
            broker(call, transport)
        }
        Op::SerialString | Op::ProductString
            if data.is_empty()
                && call.len == 0
                && if call.op == Op::SerialString {
                    policy.serial
                } else {
                    policy.product
                } =>
        {
            let Some(device) = transport.as_mut() else {
                return denied(call, "no device assigned for this request");
            };
            let result = if call.op == Op::SerialString {
                device.serial_string()
            } else {
                device.product_string()
            };
            match result {
                Ok(value)
                    if value
                        .as_ref()
                        .is_none_or(|s| s.len() <= freergb_module_api::probe::MAX_STRING) =>
                {
                    CallReply {
                        v: MODULE_API,
                        call: call.call,
                        len: Some(value.as_ref().map_or(0, |s| s.len())),
                        data: value.map(|s| encode(s.as_bytes())),
                        error: None,
                    }
                }
                Err(TransportError::Disconnected) => denied(call, "device disconnected"),
                Err(TransportError::Timeout) => denied(call, "timed out"),
                // Device/backend errors may include identity strings. Do not forward them.
                _ => denied(call, "identity string unavailable or too large"),
            }
        }
        _ => denied(call, "operation not allowed by read-only probe policy"),
    }
}

fn kind(kind: freergb_module_api::DeviceKind) -> DeviceKind {
    use freergb_module_api::DeviceKind as K;
    match kind {
        K::Keyboard | K::Keypad => DeviceKind::Keyboard,
        K::Mouse => DeviceKind::Mouse,
        K::Headset | K::HeadsetStand => DeviceKind::Headset,
        K::LightingController => DeviceKind::LightingController,
        _ => DeviceKind::Other,
    }
}

/// Opens the interface for (re)connects; called by the worker thread.
pub type Opener = Box<dyn FnMut() -> Result<Box<dyn HidTransport>, DeviceError> + Send>;

/// A device driven by a module process.
pub struct ModuleDevice {
    launch: Launch,
    interface: InterfaceInfo,
    open: Opener,
    transport: Option<Box<dyn HidTransport>>,
    process: Option<Process>,
    info: DeviceInfo,
    layout: LedLayout,
    status: DeviceStatus,
    failures: u32,
    last_error: Option<ModuleError>,
    zones: Option<Vec<Zone>>,
    platform: Option<PlatformIdentity>,

    probe: Option<freergb_module_api::probe::Policy>,
    tick_interval: Option<Duration>,
    next_tick: Option<Instant>,
}

impl ModuleDevice {
    pub fn new(
        id: DeviceId,
        name: String,
        launch: Launch,
        interface: InterfaceInfo,
        open: Opener,
    ) -> Self {
        let info = DeviceInfo {
            id,
            name,
            vendor: "Module".into(),
            kind: DeviceKind::Other,
            usb_vid: Some(interface.vid),
            usb_pid: Some(interface.pid),
            serial: None,
            firmware: None,
        };
        Self {
            launch,
            interface,
            open,
            transport: None,
            process: None,
            info,
            layout: LedLayout {
                width: 1,
                height: 1,
                leds: Vec::new(),
            },
            status: DeviceStatus::Disconnected,
            failures: 0,
            last_error: None,
            zones: None,
            platform: None,

            probe: None,
            tick_interval: None,
            next_tick: None,
        }
    }

    /// Sets optional user configuration for variable-sized zones. Omitting it
    /// preserves the module's OpenRGB-derived defaults.
    pub fn set_zones(&mut self, zones: Option<Vec<Zone>>) {
        self.zones = zones;
    }

    /// Overrides native discovery with an explicit snapshot (including an empty
    /// identity). Used by simulators; modules cannot request additional fields.
    pub fn set_platform_identity(&mut self, platform: PlatformIdentity) {
        self.platform = Some(platform);
    }
    /// Set only from the verified package manifest. None preserves legacy startup.
    pub fn set_probe_policy(&mut self, policy: Option<freergb_module_api::probe::Policy>) {
        self.probe = policy;
    }

    pub fn last_error(&self) -> Option<&ModuleError> {
        self.last_error.as_ref()
    }

    fn fault(&mut self, error: ModuleError) -> DeviceError {
        self.tick_interval = None;
        self.next_tick = None;
        self.process = None;
        self.failures += 1;
        let gone =
            error.code == ErrorCode::Disconnected && error.message.contains("device disconnected");
        if gone {
            self.transport = None;
        }
        let message = error.to_string();
        self.last_error = Some(error);
        self.status = DeviceStatus::Faulted(message.clone());
        if gone {
            DeviceError::Transport(TransportError::Disconnected)
        } else {
            DeviceError::Protocol(message)
        }
    }
}

impl RgbDevice for ModuleDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }
    fn layout(&self) -> &LedLayout {
        &self.layout
    }
    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }
    fn connect(&mut self) -> Result<(), DeviceError> {
        if self.status == DeviceStatus::Connected {
            return Ok(());
        }
        if self.failures >= MAX_CONSECUTIVE_FAILURES {
            return Err(DeviceError::Refused(format!(
                "module failed {MAX_CONSECUTIVE_FAILURES} times in a row; retry from setup"
            )));
        }
        if self.transport.is_none() {
            self.transport = Some((self.open)()?);
        }
        let mut process = match Process::spawn(&self.launch) {
            Ok(p) => p,
            Err(e) => return Err(self.fault(e)),
        };
        if let Err(e) = process.hello() {
            return Err(self.fault(e));
        }
        if let Some(policy) = &self.probe {
            match process.probe(self.interface.clone(), policy, &mut self.transport) {
                Ok(id) if id.supported => {}
                Ok(_) => {
                    return Err(self.fault(ModuleError::new(
                        ErrorCode::Unsupported,
                        "read-only identity probe refused device",
                    )))
                }
                Err(error) => {
                    let message = if error.code == ErrorCode::Disconnected {
                        "device disconnected during read-only identity probe"
                    } else {
                        "read-only identity probe failed"
                    };
                    return Err(self.fault(ModuleError::new(error.code, message)));
                }
            }
        }
        let result = process.request(
            Method::Initialize {
                interface: self.interface.clone(),
                zones: self.zones.clone(),
                platform: Some(
                    self.platform
                        .clone()
                        .unwrap_or_else(crate::platform::identity),
                ),
            },
            INITIALIZE_TIMEOUT,
            &mut self.transport,
        );
        self.tick_interval = match result
            .as_ref()
            .map_err(Clone::clone)
            .and_then(tick_interval)
        {
            Ok(interval) => interval,
            Err(e) => return Err(self.fault(e)),
        };
        self.next_tick = self.tick_interval.map(|interval| Instant::now() + interval);
        let caps: Capabilities = match result
            .and_then(|v| serde_json::from_value(v).map_err(|e| protocol(e.to_string())))
        {
            Ok(c) => c,
            Err(e) => return Err(self.fault(e)),
        };
        if caps.layout.validate().is_err()
            || caps.layout.leds.is_empty()
            || caps.layout.leds.len() > freergb_module_api::MAX_LEDS
        {
            return Err(self.fault(protocol("module reported an invalid layout")));
        }
        self.info.name = caps.name.clone();
        self.info.vendor = self.launch.module_id.clone();
        self.info.kind = kind(caps.kind);
        self.info.firmware = caps.firmware.clone();
        self.layout = caps.layout.clone();
        self.process = Some(process);
        self.status = DeviceStatus::Connected;
        self.failures = 0;
        self.last_error = None;
        Ok(())
    }
    fn poll(&mut self) -> Result<(), DeviceError> {
        if self.status != DeviceStatus::Connected
            || self.next_tick.is_none_or(|due| Instant::now() < due)
        {
            return Ok(());
        }
        let process = self.process.as_mut().ok_or(DeviceError::NotConnected)?;
        if let Err(e) = process.request(Method::Tick, REQUEST_TIMEOUT, &mut self.transport) {
            return Err(self.fault(e));
        }
        // Schedule from completion; never catch up with a burst after a slow request.
        self.next_tick = self.tick_interval.map(|interval| Instant::now() + interval);
        Ok(())
    }
    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        if self.status != DeviceStatus::Connected {
            return Err(DeviceError::NotConnected);
        }
        check_frame(&self.layout, frame)?;
        let rgb = pack_rgb(frame.colors());
        let process = self.process.as_mut().ok_or(DeviceError::NotConnected)?;
        match process.request(Method::Frame { rgb }, REQUEST_TIMEOUT, &mut self.transport) {
            Ok(_) => Ok(()),
            Err(e) => Err(self.fault(e)),
        }
    }
    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError> {
        self.tick_interval = None;
        self.next_tick = None;
        let result = match self.process.as_mut() {
            Some(process) => process
                .request(
                    Method::Shutdown { final_color },
                    REQUEST_TIMEOUT,
                    &mut self.transport,
                )
                .map(|_| ())
                .map_err(|e| DeviceError::Protocol(e.to_string())),
            None => Ok(()),
        };
        self.process = None;
        self.status = DeviceStatus::Disconnected;
        result
    }
}
#[cfg(test)]
mod probe_tests {
    use super::*;
    use freergb_module_api::probe::{Feature, Policy};
    use std::sync::{Arc, Mutex};

    struct Simulated(Arc<Mutex<Vec<String>>>);
    impl HidTransport for Simulated {
        fn write(&mut self, _: &[u8]) -> Result<usize, TransportError> {
            panic!("probe write")
        }
        fn send_feature(&mut self, _: &[u8]) -> Result<usize, TransportError> {
            panic!("probe feature write")
        }
        fn read_timeout(&mut self, _: &mut [u8], _: i32) -> Result<usize, TransportError> {
            panic!("probe input read")
        }
        fn output_report_len(&self) -> usize {
            64
        }
        fn input_report_len(&self) -> usize {
            64
        }
        fn serial_string(&mut self) -> Result<Option<String>, TransportError> {
            self.0.lock().unwrap().push("serial".into());
            Ok(Some("private-serial".into()))
        }
        fn product_string(&mut self) -> Result<Option<String>, TransportError> {
            self.0.lock().unwrap().push("product".into());
            Ok(None)
        }
        fn get_feature(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
            assert_eq!(buf, &[0x52, 0, 0, 0]);
            self.0.lock().unwrap().push("feature".into());
            buf[1] = 42;
            Ok(2)
        }
    }
    fn policy() -> Policy {
        Policy {
            version: 1,
            serial: true,
            product: true,
            features: vec![Feature {
                report_id: 0x52,
                max_len: 4,
            }],
        }
    }
    fn call(op: Op, bytes: &[u8], len: usize) -> Call {
        Call {
            v: MODULE_API,
            call: 4,
            op,
            data: encode(bytes),
            len,
            timeout_ms: 0,
        }
    }

    #[test]
    fn untrusted_probe_calls_are_checked_before_transport_access() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut transport = Some(Box::new(Simulated(seen.clone())) as Box<dyn HidTransport>);
        let mut calls = vec![
            call(Op::Write, &[0, 1], 0),
            call(Op::SendFeature, &[0], 0),
            call(Op::Read, &[], 4),
            call(Op::Delay, &[], 0),
            call(Op::GetFeature, &[0x53], 4),
            call(Op::GetFeature, &[0x52], 5),
            call(Op::GetFeature, &[0x52], 0),
            call(Op::GetFeature, &[0x52, 1], 4),
            call(Op::GetFeature, &[], 4),
            call(Op::SerialString, &[1], 0),
            call(Op::ProductString, &[], 1),
        ];
        let mut bad_version = call(Op::SerialString, &[], 0);
        bad_version.v = 2;
        calls.push(bad_version);
        let mut bad_base64 = call(Op::SerialString, &[], 0);
        bad_base64.data = "!".into();
        calls.push(bad_base64);
        for c in calls {
            assert!(
                broker_probe(&c, &policy(), &mut transport).error.is_some(),
                "{c:?}"
            );
        }
        let denied_policy = Policy {
            serial: false,
            product: false,
            features: vec![],
            ..policy()
        };
        for op in [Op::SerialString, Op::ProductString] {
            assert!(
                broker_probe(&call(op, &[], 0), &denied_policy, &mut transport)
                    .error
                    .is_some()
            );
            assert!(
                broker(&call(op, &[], 0), &mut transport).error.is_some(),
                "not outside probe"
            );
        }
        assert!(seen.lock().unwrap().is_empty());
        let reply = broker_probe(&call(Op::SerialString, &[], 0), &policy(), &mut transport);
        assert_eq!(
            decode(reply.data.as_deref().unwrap()).unwrap(),
            b"private-serial"
        );
        assert_eq!(reply.len, Some(14));
        let reply = broker_probe(&call(Op::ProductString, &[], 0), &policy(), &mut transport);
        assert_eq!(reply.data, None);
        let reply = broker_probe(&call(Op::GetFeature, &[0x52], 4), &policy(), &mut transport);
        assert_eq!(decode(reply.data.as_deref().unwrap()).unwrap(), [0x52, 42]);
        assert_eq!(reply.len, Some(2));
        assert_eq!(*seen.lock().unwrap(), ["serial", "product", "feature"]);
    }
}

#[cfg(test)]
mod timing_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn old_capabilities_need_no_timing_and_extension_is_versioned() {
        assert_eq!(tick_interval(&json!({})).unwrap(), None);
        assert_eq!(
            tick_interval(&json!({"timing":{"v":1,"tick_ms":null}})).unwrap(),
            None
        );
        for ms in [500, 10_000] {
            assert_eq!(
                tick_interval(&json!({"timing":{"v":1,"tick_ms":ms}})).unwrap(),
                Some(Duration::from_millis(ms))
            );
        }
        for timing in [
            json!(null),
            json!({"v":2,"tick_ms":500}),
            json!({"v":1,"tick_ms":499}),
            json!({"v":1,"tick_ms":10001}),
            json!({"v":1,"tick_ms":-1}),
            json!({"v":1,"tick_ms":500.5}),
            json!({"v":1,"tick_ms":"500"}),
        ] {
            assert!(tick_interval(&json!({"timing":timing})).is_err());
        }
    }

    fn delay(ms: i32) -> Call {
        Call {
            v: MODULE_API,
            call: 1,
            op: Op::Delay,
            data: String::new(),
            len: 0,
            timeout_ms: ms,
        }
    }

    #[test]
    fn delay_bounds_budget_shape_and_deadline_are_checked_before_sleeping() {
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        for ms in [-1, 0, 101, i32::MAX] {
            assert!(sequence_delay(&delay(ms), &mut 0, deadline).is_err());
        }
        assert_eq!(
            sequence_delay(&delay(13), &mut 0, deadline).unwrap(),
            Duration::from_millis(13)
        );
        let mut total = 900;
        sequence_delay(&delay(100), &mut total, deadline).unwrap();
        assert!(sequence_delay(&delay(1), &mut total, deadline).is_err());
        let mut call = delay(13);
        call.data = "AA==".into();
        assert!(sequence_delay(&call, &mut 0, deadline).is_err());
        call.data.clear();
        call.len = 1;
        assert!(sequence_delay(&call, &mut 0, deadline).is_err());
        assert_eq!(
            sequence_delay(&delay(13), &mut 0, Instant::now())
                .unwrap_err()
                .code,
            ErrorCode::Timeout
        );
        assert!(broker(&delay(13), &mut None).error.is_some());
    }
}
