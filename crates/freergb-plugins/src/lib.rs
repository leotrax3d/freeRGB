//! Explicitly trusted native plugins, isolated for faults, not sandboxed.
//! Each manifest describes one device and launches one child without a shell.
use freergb_core::{Frame, LedLayout, Rgb};
use freergb_device_api::{
    check_frame, DeviceError, DeviceId, DeviceInfo, DeviceKind, DeviceStatus, RgbDevice,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::runtime::Runtime;

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_MESSAGE: usize = 256 * 1024;
pub const MAX_LEDS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Virtual,
    Hardware,
}

/// Relative executable paths resolve against the manifest, never PATH.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub api_version: u32,
    pub id: String,
    pub name: String,
    pub executable: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    pub access: Access,
    pub layout: LedLayout,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    1000
}
fn invalid(message: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

impl Manifest {
    pub fn load(path: &Path) -> io::Result<Self> {
        let path = path.canonicalize()?;
        let mut bytes = Vec::new();
        std::fs::File::open(&path)?
            .take((MAX_MESSAGE + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_MESSAGE {
            return Err(invalid("manifest too large"));
        }
        let mut manifest: Self = serde_json::from_slice(&bytes).map_err(invalid)?;
        if !manifest.executable.is_absolute() {
            manifest.executable = path.parent().unwrap().join(&manifest.executable);
        }
        manifest.executable = manifest.executable.canonicalize()?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.api_version != PROTOCOL_VERSION {
            return Err(invalid("unsupported plugin API version"));
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(invalid("plugin id must be 1..64 ASCII letters/digits/-/_"));
        }
        if self.name.trim().is_empty() || self.name.len() > 128 {
            return Err(invalid("invalid plugin name"));
        }
        if !self.executable.is_absolute() || !self.executable.is_file() {
            return Err(invalid("plugin executable must be an absolute file path"));
        }
        if self.args.len() > 32 || self.args.iter().any(|a| a.len() > 4096 || a.contains('\0')) {
            return Err(invalid("invalid plugin arguments"));
        }
        if !(50..=5000).contains(&self.timeout_ms) {
            return Err(invalid("timeout_ms must be 50..5000"));
        }
        self.layout.validate().map_err(invalid)?;
        if self.layout.is_empty()
            || self.layout.len() > MAX_LEDS
            || self.layout.leds.iter().any(|l| l.name.len() > 128)
        {
            return Err(invalid("layout must contain 1..4096 LEDs with short names"));
        }
        // The handshake includes the layout and must also fit the wire limit.
        if serde_json::to_vec(&json!({"api_version": PROTOCOL_VERSION, "id": u64::MAX, "method": "initialize", "params": {"layout": self.layout}})).map_err(invalid)?.len() >= MAX_MESSAGE {
            return Err(invalid("layout exceeds protocol message limit"));
        }
        Ok(())
    }

    pub fn device_id(&self) -> DeviceId {
        DeviceId(format!("plugin:{}", self.id))
    }
}

struct Session {
    runtime: Runtime,
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
    timeout: Duration,
}

impl Session {
    fn start(manifest: &Manifest) -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let mut command = Command::new(&manifest.executable);
        command
            .args(&manifest.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let mut child = {
            let _enter = runtime.enter();
            command.spawn()?
        };
        let input = child
            .stdin
            .take()
            .ok_or_else(|| invalid("missing plugin stdin"))?;
        let output = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| invalid("missing plugin stdout"))?,
        );
        Ok(Self {
            runtime,
            child,
            input,
            output,
            next_id: 0,
            timeout: Duration::from_millis(manifest.timeout_ms),
        })
    }

    fn request(&mut self, method: &str, params: Value) -> io::Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = serde_json::to_vec(
            &json!({"api_version": PROTOCOL_VERSION, "id": id, "method": method, "params": params}),
        )
        .map_err(invalid)?;
        if message.len() >= MAX_MESSAGE {
            return Err(invalid("request too large"));
        }
        message.push(b'\n');
        self.runtime.block_on(async {
            tokio::time::timeout(self.timeout, async {
                self.input.write_all(&message).await?;
                self.input.flush().await?;
                let mut response = Vec::new();
                (&mut self.output)
                    .take((MAX_MESSAGE + 1) as u64)
                    .read_until(b'\n', &mut response)
                    .await?;
                if response.len() > MAX_MESSAGE || response.last() != Some(&b'\n') {
                    return Err(invalid("oversized or incomplete plugin reply"));
                }
                let reply: Value = serde_json::from_slice(&response).map_err(invalid)?;
                if reply.get("api_version").and_then(Value::as_u64)
                    != Some(u64::from(PROTOCOL_VERSION))
                    || reply.get("id").and_then(Value::as_u64) != Some(id)
                {
                    return Err(invalid("plugin reply version/id mismatch"));
                }
                if let Some(error) = reply.get("error") {
                    return Err(invalid(format!("plugin error: {error}")));
                }
                reply
                    .get("result")
                    .cloned()
                    .ok_or_else(|| invalid("missing plugin result"))
            })
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "plugin request timed out"))?
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), self.child.wait()).await
        });
    }
}

pub struct ProcessDevice {
    manifest: Manifest,
    info: DeviceInfo,
    session: Option<Session>,
    status: DeviceStatus,
}

impl ProcessDevice {
    pub fn new(manifest: Manifest) -> io::Result<Self> {
        manifest.validate()?;
        let info = DeviceInfo {
            id: manifest.device_id(),
            name: manifest.name.clone(),
            vendor: "Process plugin".into(),
            kind: if manifest.access == Access::Virtual {
                DeviceKind::Virtual
            } else {
                DeviceKind::Other
            },
            usb_vid: None,
            usb_pid: None,
            serial: None,
            firmware: None,
        };
        Ok(Self {
            manifest,
            info,
            session: None,
            status: DeviceStatus::Disconnected,
        })
    }

    fn fault(&mut self, error: impl ToString) -> DeviceError {
        let message = error.to_string();
        self.session = None;
        self.status = DeviceStatus::Faulted(message.clone());
        DeviceError::Protocol(message)
    }

    fn exchange(&mut self, method: &str, params: Value) -> Result<(), DeviceError> {
        let result = self
            .session
            .as_mut()
            .ok_or(DeviceError::NotConnected)?
            .request(method, params);
        match result {
            Ok(Value::Bool(true)) => Ok(()),
            Ok(_) => Err(self.fault("plugin must acknowledge with result:true")),
            Err(error) => Err(self.fault(error)),
        }
    }
}

impl RgbDevice for ProcessDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }
    fn layout(&self) -> &LedLayout {
        &self.manifest.layout
    }
    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }
    fn connect(&mut self) -> Result<(), DeviceError> {
        if self.status == DeviceStatus::Connected {
            return Ok(());
        }
        self.session = Some(Session::start(&self.manifest).map_err(|e| self.fault(e))?);
        self.exchange("initialize", json!({"layout": self.manifest.layout}))?;
        self.status = DeviceStatus::Connected;
        Ok(())
    }
    fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        if self.status != DeviceStatus::Connected {
            return Err(DeviceError::NotConnected);
        }
        check_frame(self.layout(), frame)?;
        self.exchange("frame", json!({"colors": frame.colors()}))
    }
    fn disconnect(&mut self, final_color: Option<Rgb>) -> Result<(), DeviceError> {
        let result = if self.session.is_some() {
            self.exchange("shutdown", json!({"final_color": final_color}))
        } else {
            Ok(())
        };
        self.session = None;
        if result.is_ok() {
            self.status = DeviceStatus::Disconnected;
        }
        result
    }
}
