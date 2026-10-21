//! User-side Rust transport to the authenticated protected service, through the
//! bundled C# relay (which opens the restricted pipe without generic-write rights).
use crate::native_session::NativeSessionClient;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, BufReader, ReadBuf};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub struct NativeBridge {
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    _child: Child,
}
impl AsyncRead for NativeBridge {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.output).poll_read(cx, buffer)
    }
}
impl AsyncWrite for NativeBridge {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.input).poll_write(cx, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.input).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.input).poll_shutdown(cx)
    }
}

pub async fn connect(helper: PathBuf) -> Result<NativeSessionClient<NativeBridge>, String> {
    if !cfg!(windows) {
        return Err("Native mainboard bridge is Windows-only".into());
    }
    let helper = helper
        .canonicalize()
        .map_err(|_| "Mainboard helper not found")?;
    let mut command = Command::new(helper);
    command
        .arg("--control-relay")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command
        .spawn()
        .map_err(|error| format!("Native relay startup failed: {error}"))?;
    let input = child.stdin.take().ok_or("Native relay stdin unavailable")?;
    let mut output = BufReader::new(
        child
            .stdout
            .take()
            .ok_or("Native relay stdout unavailable")?,
    );
    let mut ready = Vec::new();
    tokio::time::timeout(
        Duration::from_secs(11),
        (&mut output).take(64).read_until(b'\n', &mut ready),
    )
    .await
    .map_err(|_| "Protected session startup deadline exceeded")?
    .map_err(|_| "Protected session refused or disconnected")?;
    if ready != b"ready\n" && ready != b"ready\r\n" {
        return Err(
            "Protected session not ready; check protected commissioning authorization".into(),
        );
    }
    NativeSessionClient::from_authenticated_transport(NativeBridge {
        input,
        output,
        _child: child,
    })
    .await
}
