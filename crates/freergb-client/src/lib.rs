//! Blocking client for the freergbd JSON-RPC API (see `docs/LOCAL_API.md`).
//!
//! Each call opens a fresh connection, sends one line and reads one line. This
//! keeps the client stateless and robust against daemon restarts.

use serde_json::{json, Value};
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

const REQUEST_LIMIT: usize = 256 * 1024;
const RESPONSE_LIMIT: usize = 16 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(3);

fn invalid(message: &str) -> ClientError {
    ClientError::Io(io::Error::new(io::ErrorKind::InvalidData, message))
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("cannot reach freergbd (is it running?): {0}")]
    Connect(std::io::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("daemon error {code}: {message}")]
    Rpc { code: i64, message: String },
}

#[cfg(windows)]
pub const PIPE_NAME: &str = r"\\.\pipe\freergb";

#[cfg(windows)]
async fn connect() -> Result<tokio::net::windows::named_pipe::NamedPipeClient, ClientError> {
    loop {
        match tokio::net::windows::named_pipe::ClientOptions::new().open(PIPE_NAME) {
            Ok(pipe) => return Ok(pipe),
            Err(error) if error.raw_os_error() == Some(231) => {
                tokio::time::sleep(Duration::from_millis(10)).await
            }
            Err(error) => return Err(ClientError::Connect(error)),
        }
    }
}

#[cfg(unix)]
async fn connect() -> Result<tokio::net::UnixStream, ClientError> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    tokio::net::UnixStream::connect(dir.join("freergb.sock"))
        .await
        .map_err(ClientError::Connect)
}

async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    request: &[u8],
    limit: usize,
) -> Result<Value, ClientError> {
    stream.write_all(request).await?;
    stream.flush().await?;
    let mut bytes = Vec::new();
    BufReader::new(stream)
        .take((limit + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .await?;
    if bytes.len() > limit {
        return Err(invalid("daemon response exceeds size limit"));
    }
    if bytes.last() != Some(&b'\n') {
        return Err(invalid("incomplete daemon response"));
    }
    let reply: Value = serde_json::from_slice(&bytes)?;
    if reply["jsonrpc"] != "2.0"
        || reply["id"] != 1
        || reply.get("result").is_some() == reply.get("error").is_some()
    {
        return Err(invalid("invalid JSON-RPC response envelope"));
    }
    if let Some(error) = reply.get("error") {
        if error["code"].as_i64().is_none() || error["message"].as_str().is_none() {
            return Err(invalid("invalid JSON-RPC error"));
        }
    }
    Ok(reply)
}

async fn timed<T>(
    future: impl std::future::Future<Output = Result<T, ClientError>>,
    timeout: Duration,
) -> Result<T, ClientError> {
    tokio::time::timeout(timeout, future).await.map_err(|_| {
        ClientError::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            "daemon request timed out",
        ))
    })?
}

/// Blocking call, with a 3-second total connect/write/read deadline. Use
/// `spawn_blocking` if calling from an asynchronous runtime.
pub fn call_raw(method: &str, params: Value) -> Result<Value, ClientError> {
    let mut request =
        serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))?;
    request.push(b'\n');
    if request.len() > REQUEST_LIMIT {
        return Err(invalid("request exceeds daemon size limit"));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(timed(
        async {
            let stream = connect().await?;
            exchange(stream, &request, RESPONSE_LIMIT).await
        },
        DEADLINE,
    ))
}

/// Calls `method` and returns its `result`, mapping RPC errors to `Err`.
pub fn call(method: &str, params: Value) -> Result<Value, ClientError> {
    let resp = call_raw(method, params)?;
    if let Some(e) = resp.get("error") {
        return Err(ClientError::Rpc {
            code: e.get("code").and_then(Value::as_i64).unwrap_or(0),
            message: e
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
        });
    }
    Ok(resp.get("result").cloned().unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[tokio::test]
    async fn real_named_pipe_with_silent_server_times_out() {
        use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
        let name = format!(
            r"\\.\pipe\freergb-client-deadline-test-{}",
            std::process::id()
        );
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)
            .unwrap();
        let client = ClientOptions::new().open(&name).unwrap();
        server.connect().await.unwrap();
        let error = timed(
            exchange(client, b"request\n", 1024),
            Duration::from_millis(30),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error,ClientError::Io(ref error) if error.kind()==io::ErrorKind::TimedOut)
        );
    }
    #[tokio::test]
    async fn valid_response_and_bad_envelopes() {
        for (reply, valid) in [
            ("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":true}\n", true),
            ("{\"jsonrpc\":\"2.0\",\"id\":99,\"result\":true}\n", false),
            (
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":true,\"error\":{}}\n",
                false,
            ),
        ] {
            let (client, mut server) = tokio::io::duplex(1024);
            tokio::spawn(async move {
                let _ = server.write_all(reply.as_bytes()).await;
            });
            assert_eq!(exchange(client, b"request\n", 1024).await.is_ok(), valid);
        }
    }
    #[tokio::test]
    async fn stalled_daemon_and_nonreading_peer_cannot_block_forever() {
        for request in [vec![b'x'; 8], vec![b'x'; 4096]] {
            let (client, _server) = tokio::io::duplex(16);
            let error = timed(exchange(client, &request, 1024), Duration::from_millis(30))
                .await
                .unwrap_err();
            assert!(
                matches!(error,ClientError::Io(ref error) if error.kind()==io::ErrorKind::TimedOut)
            );
        }
    }
    #[tokio::test]
    async fn oversized_unterminated_response_is_bounded() {
        let (client, mut server) = tokio::io::duplex(1024);
        tokio::spawn(async move {
            let _ = server.write_all(&[b'x'; 65]).await;
        });
        let error = exchange(client, b"request\n", 64).await.unwrap_err();
        assert!(error.to_string().contains("size limit"));
    }
}
