//! Trusted interpreter child. The untrusted WASM has no imports or OS access.
use crate::runtime::WasmEffect;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use freergb_core::{Canvas, Rgb};
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    runtime::Runtime,
};

const VERSION: u32 = 1;
const MAX_MESSAGE: usize = 768 * 1024;
fn invalid(e: impl ToString) -> io::Error {
    io::Error::other(e.to_string())
}

#[derive(Clone)]
pub struct HostCommand {
    pub executable: PathBuf,
    pub args: Vec<String>,
}
impl HostCommand {
    pub fn sibling() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        if exe.file_stem().is_some_and(|n| n == "freergbd") {
            return Ok(Self {
                executable: exe,
                args: vec!["--effect-worker".into()],
            });
        }
        let mut dir = exe
            .parent()
            .ok_or_else(|| invalid("Missing executable directory"))?;
        if dir
            .file_name()
            .is_some_and(|n| n == "deps" || n == "examples")
        {
            dir = dir
                .parent()
                .ok_or_else(|| invalid("Missing build directory"))?;
        }
        Ok(Self {
            executable: dir.join(format!(
                "freergb-effect-worker{}",
                std::env::consts::EXE_SUFFIX
            )),
            args: vec![],
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    api: u32,
    id: u64,
    command: Operation,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "method", deny_unknown_fields)]
enum Operation {
    Load {
        wasm: String,
        parameters: Vec<f64>,
    },
    Render {
        seconds: f64,
        width: usize,
        height: usize,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    api: u32,
    id: u64,
    result: Result<String, String>,
}

struct ProcessClient {
    runtime: Runtime,
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    id: u64,
}
impl ProcessClient {
    pub fn load(host: &HostCommand, bytes: &[u8], parameters: &[f64]) -> Result<Self, String> {
        if bytes.len() > crate::runtime::MAX_WASM
            || parameters.len() > 16
            || parameters.iter().any(|p| !p.is_finite())
        {
            return Err("Invalid effect package or parameters".into());
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&host.executable);
        cmd.args(&host.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        let mut child = {
            let _enter = runtime.enter();
            cmd.spawn()
                .map_err(|e| format!("Cannot start effect host: {e}"))?
        };
        let input = child.stdin.take().ok_or("Missing effect stdin")?;
        let output = BufReader::new(child.stdout.take().ok_or("Missing effect stdout")?);
        let mut effect = Self {
            runtime,
            child,
            input,
            output,
            id: 0,
        };
        let result = effect.exchange(
            Operation::Load {
                wasm: STANDARD.encode(bytes),
                parameters: parameters.to_vec(),
            },
            Duration::from_secs(3),
        )?;
        if !result.is_empty() {
            return Err("Invalid effect initialization reply".into());
        }
        // Activation and reload are transactional, including the first frame.
        effect.frame(Duration::ZERO, &mut Canvas::new(128, 64))?;
        Ok(effect)
    }
    fn exchange(&mut self, command: Operation, deadline: Duration) -> Result<String, String> {
        self.id = self
            .id
            .checked_add(1)
            .ok_or("Effect request counter exhausted")?;
        let id = self.id;
        let mut bytes = serde_json::to_vec(&Request {
            api: VERSION,
            id,
            command,
        })
        .map_err(|e| e.to_string())?;
        if bytes.len() >= MAX_MESSAGE {
            return Err("Effect request too large".into());
        }
        bytes.push(b'\n');
        self.runtime
            .block_on(async {
                tokio::time::timeout(deadline, async {
                    self.input.write_all(&bytes).await?;
                    self.input.flush().await?;
                    let mut bytes = Vec::new();
                    (&mut self.output)
                        .take((MAX_MESSAGE + 1) as u64)
                        .read_until(b'\n', &mut bytes)
                        .await?;
                    if bytes.len() > MAX_MESSAGE || bytes.last() != Some(&b'\n') {
                        return Err(invalid("Incomplete or oversized effect reply"));
                    }
                    let reply: Reply = serde_json::from_slice(&bytes).map_err(invalid)?;
                    if reply.api != VERSION || reply.id != id {
                        return Err(invalid("Effect reply version/id mismatch"));
                    }
                    reply.result.map_err(invalid)
                })
                .await
                .map_err(|_| invalid("Effect host deadline exceeded"))?
            })
            .map_err(|e| e.to_string())
    }
    pub fn frame(&mut self, time: Duration, canvas: &mut Canvas) -> Result<(), String> {
        let (w, h) = (canvas.width().min(128), canvas.height().min(64));
        let data = self.exchange(
            Operation::Render {
                seconds: time.as_secs_f64(),
                width: w,
                height: h,
            },
            Duration::from_millis(250),
        )?;
        let data = STANDARD.decode(data).map_err(|e| e.to_string())?;
        if data.len() != w * h * 3 {
            return Err("Invalid effect frame length".into());
        }
        canvas.paint(|x, y| {
            let i = (((y * h as f32) as usize).min(h - 1) * w
                + ((x * w as f32) as usize).min(w - 1))
                * 3;
            Rgb::new(data[i], data[i + 1], data[i + 2])
        });
        Ok(())
    }
}
impl Drop for ProcessClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), self.child.wait()).await
        });
    }
}

/// Run before daemon initialization: no hardware, API listener, logging or state.
pub fn serve() -> io::Result<()> {
    use std::io::{BufRead, Read, Write};
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut effect = None;
    let mut last_id = 0;
    loop {
        let mut bytes = Vec::new();
        (&mut input)
            .take((MAX_MESSAGE + 1) as u64)
            .read_until(b'\n', &mut bytes)?;
        if bytes.is_empty() {
            return Ok(());
        }
        if bytes.len() > MAX_MESSAGE || bytes.last() != Some(&b'\n') {
            return Err(invalid("Invalid effect request size"));
        }
        let request: Request = serde_json::from_slice(&bytes).map_err(invalid)?;
        if request.api != VERSION || request.id <= last_id {
            return Err(invalid("Invalid effect request version/id"));
        }
        last_id = request.id;
        let result = (|| -> Result<String, String> {
            match request.command {
                Operation::Load { wasm, parameters } => {
                    if effect.is_some() {
                        return Err("Effect already initialized".into());
                    }
                    let bytes = STANDARD.decode(wasm).map_err(|e| e.to_string())?;
                    effect = Some(WasmEffect::load(&bytes, &parameters, WasmEffect::fault())?);
                    Ok(String::new())
                }
                Operation::Render {
                    seconds,
                    width,
                    height,
                } => {
                    if !seconds.is_finite()
                        || seconds < 0.0
                        || !(1..=128).contains(&width)
                        || !(1..=64).contains(&height)
                    {
                        return Err("Invalid frame dimensions/time".into());
                    }
                    let time = Duration::try_from_secs_f64(seconds).map_err(|e| e.to_string())?;
                    let effect = effect.as_mut().ok_or("Effect not initialized")?;
                    let mut canvas = Canvas::new(width, height);
                    effect.frame(time, &mut canvas)?;
                    let mut bytes = Vec::with_capacity(width * height * 3);
                    for y in 0..height {
                        for x in 0..width {
                            let c = canvas.get(x, y).unwrap();
                            bytes.extend_from_slice(&[c.r, c.g, c.b]);
                        }
                    }
                    Ok(STANDARD.encode(bytes))
                }
            }
        })();
        let failed = result.is_err();
        serde_json::to_writer(
            &mut output,
            &Reply {
                api: VERSION,
                id: request.id,
                result,
            },
        )
        .map_err(invalid)?;
        output.write_all(b"\n")?;
        output.flush()?;
        if failed {
            return Ok(());
        }
    }
}
// Keep runtime creation, block_on and destruction off async IPC threads.
type FrameReply = std::sync::mpsc::SyncSender<Result<Canvas, String>>;
type FrameRequest = (Duration, usize, usize, FrameReply);
pub struct ProcessEffect {
    frames: Option<std::sync::mpsc::SyncSender<FrameRequest>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ProcessEffect {
    pub fn load(host: &HostCommand, bytes: &[u8], parameters: &[f64]) -> Result<Self, String> {
        let (frames, requests) = std::sync::mpsc::sync_channel::<FrameRequest>(1);
        let (ready, initialized) = std::sync::mpsc::sync_channel(1);
        let (host, bytes, parameters) = (host.clone(), bytes.to_vec(), parameters.to_vec());
        let thread = std::thread::Builder::new()
            .name("effect-process".into())
            .spawn(move || {
                let mut client = match ProcessClient::load(&host, &bytes, &parameters) {
                    Ok(client) => {
                        let _ = ready.send(Ok(()));
                        client
                    }
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                while let Ok((time, width, height, reply)) = requests.recv() {
                    let mut canvas = Canvas::new(width, height);
                    let result = client.frame(time, &mut canvas).map(|()| canvas);
                    let failed = result.is_err();
                    let _ = reply.send(result);
                    if failed {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let effect = Self {
            frames: Some(frames),
            thread: Some(thread),
        };
        initialized.recv().map_err(|e| e.to_string())??;
        Ok(effect)
    }
    pub fn frame(&mut self, time: Duration, canvas: &mut Canvas) -> Result<(), String> {
        let (reply, result) = std::sync::mpsc::sync_channel(1);
        self.frames
            .as_ref()
            .ok_or("Effect stopped")?
            .send((
                time,
                canvas.width().min(128),
                canvas.height().min(64),
                reply,
            ))
            .map_err(|_| "Effect host exited")?;
        let frame = result.recv().map_err(|_| "Effect host exited")??;
        canvas.paint(|x, y| frame.sample(x, y));
        Ok(())
    }
}
impl Drop for ProcessEffect {
    fn drop(&mut self) {
        self.frames.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
