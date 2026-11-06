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
        loop {
            let message = self.receive(deadline)?;
            if message.get("call").is_some() {
                let call: Call =
                    serde_json::from_value(message).map_err(|e| protocol(e.to_string()))?;
                calls += 1;
                if call.v != MODULE_API || calls > MAX_CALLS_PER_REQUEST {
                    return Err(protocol("invalid or excessive transport call"));
                }
                let reply = broker(&call, transport);
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
        Op::Write => transport.write(&data).map(|n| (None, n)),
        Op::SendFeature => transport.send_feature(&data).map(|n| (None, n)),
        Op::Read => {
            let mut buffer = vec![0u8; call.len.max(1)];
            transport
                .read_timeout(&mut buffer, call.timeout_ms.clamp(0, 1000))
                .map(|n| (Some(buffer[..n].to_vec()), n))
        }
        Op::GetFeature => {
            let mut buffer = vec![0u8; call.len.max(1)];
            let prefix = data.len().min(buffer.len());
            buffer[..prefix].copy_from_slice(&data[..prefix]);
            transport
                .get_feature(&mut buffer)
                .map(|n| (Some(buffer[..n].to_vec()), n))
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

    pub fn last_error(&self) -> Option<&ModuleError> {
        self.last_error.as_ref()
    }

    fn fault(&mut self, error: ModuleError) -> DeviceError {
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
