//! Blocking client for the freergbd JSON-RPC API (see `docs/LOCAL_API.md`).
//!
//! Each call opens a fresh connection, sends one line and reads one line. This
//! keeps the client stateless and robust against daemon restarts.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};

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
fn connect() -> Result<std::fs::File, ClientError> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE_NAME)
        .map_err(ClientError::Connect)
}

#[cfg(unix)]
fn connect() -> Result<std::os::unix::net::UnixStream, ClientError> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    std::os::unix::net::UnixStream::connect(dir.join("freergb.sock")).map_err(ClientError::Connect)
}

/// Calls `method` and returns the full JSON-RPC response object.
pub fn call_raw(method: &str, params: Value) -> Result<Value, ClientError> {
    let stream = connect()?;
    let mut writer = stream.try_clone()?;
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    writeln!(writer, "{req}")?;
    writer.flush()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    Ok(serde_json::from_str(&line)?)
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
