//! Local IPC transport for the JSON-RPC API. No network listener.
//!
//! * Windows: named pipe `\\.\pipe\freergb`. Remote clients are rejected and
//!   the default pipe DACL only grants write access to the creating user,
//!   LocalSystem and administrators.
//! * Unix: socket `$XDG_RUNTIME_DIR/freergb.sock` (fallback `/tmp`), mode 0600.

use crate::{api, SharedEngine};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::Notify;

/// Includes the terminating newline. Oversized requests close the connection.
pub const MAX_REQUEST_BYTES: usize = 256 * 1024;

#[cfg(windows)]
pub const PIPE_NAME: &str = r"\\.\pipe\freergb";

#[cfg(unix)]
pub fn socket_path() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("freergb.sock")
}

/// Serves one client connection until EOF.
async fn serve_conn<S>(stream: S, engine: SharedEngine, shutdown: Arc<Notify>)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (r, mut w) = tokio::io::split(stream);
    let mut reader = BufReader::new(r);
    loop {
        let mut bytes = Vec::new();
        let read = (&mut reader)
            .take((MAX_REQUEST_BYTES + 1) as u64)
            .read_until(b'\n', &mut bytes)
            .await;
        if read.is_err() || bytes.is_empty() {
            break;
        }
        if bytes.len() > MAX_REQUEST_BYTES {
            let _ = w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32600,\"message\":\"request exceeds 262144 bytes\"}}\n").await;
            break;
        }
        if bytes.last() != Some(&b'\n') {
            break;
        }
        let Ok(line) = String::from_utf8(bytes) else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        // The handler may block briefly on the engine mutex during a frame.
        let e = engine.clone();
        let reply = match tokio::task::spawn_blocking(move || api::handle_line(&e, &line)).await {
            Ok(r) => r,
            Err(_) => break,
        };
        if w.write_all(reply.line.as_bytes()).await.is_err() || w.write_all(b"\n").await.is_err() {
            break;
        }
        let _ = w.flush().await;
        if reply.shutdown {
            shutdown.notify_one();
        }
    }
}

#[cfg(windows)]
pub async fn serve(engine: SharedEngine, shutdown: Arc<Notify>) -> std::io::Result<()> {
    serve_named(engine, shutdown, PIPE_NAME).await
}

/// Separate endpoint for hardware-free integration tests alongside a GUI preview.
/// Normal clients and the public default endpoint remain unchanged.
#[cfg(windows)]
pub fn validate_instance(instance: &str) -> Result<String, String> {
    if instance.is_empty()
        || instance.len() > 64
        || !instance
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("IPC instance must be 1..64 ASCII letters/digits/hyphens".into());
    }
    Ok(instance.into())
}

#[cfg(windows)]
pub async fn serve_instance(
    engine: SharedEngine,
    shutdown: Arc<Notify>,
    instance: &str,
) -> std::io::Result<()> {
    let instance = validate_instance(instance)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    serve_named(engine, shutdown, &format!(r"\\.\pipe\freergb-{instance}")).await
}

#[cfg(windows)]
async fn serve_named(
    engine: SharedEngine,
    shutdown: Arc<Notify>,
    name: &str,
) -> std::io::Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;

    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(name)?;
    tracing::info!(pipe = name, "API listening");
    loop {
        server.connect().await?;
        let client = server;
        server = ServerOptions::new()
            .reject_remote_clients(true)
            .create(name)?;
        tokio::spawn(serve_conn(client, engine.clone(), shutdown.clone()));
    }
}

#[cfg(unix)]
fn bind_socket(
    path: &std::path::Path,
) -> std::io::Result<(tokio::net::UnixListener, std::fs::File)> {
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::fs::PermissionsExt;

    // Hold this OS lock for the listener's whole lifetime. It also serializes
    // stale-socket recovery, so simultaneous starts cannot unlink each other.
    // Keep the lock file after shutdown: removing it would permit two lock inodes.
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path.with_extension("sock.lock"))?;
    lock.try_lock()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::AddrInUse, error))?;
    // A second daemon must not unlink a live endpoint or an unrelated file.
    // Recover only a socket whose previous listener has actually gone away.
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => {
            match std::os::unix::net::UnixStream::connect(path) {
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                    std::fs::remove_file(path)?;
                }
                Err(error) => return Err(error),
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "freeRGB API is already running",
                    ))
                }
            }
        }
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "refusing to replace a non-socket API path",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let listener = tokio::net::UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok((listener, lock))
}

#[cfg(unix)]
pub async fn serve(engine: SharedEngine, shutdown: Arc<Notify>) -> std::io::Result<()> {
    let path = socket_path();
    let (listener, _endpoint_lock) = bind_socket(&path)?;
    tracing::info!(socket = %path.display(), "API listening");
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(serve_conn(stream, engine.clone(), shutdown.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use freergb_core::Rgb;
    use freergb_effects::EffectSpec;
    use std::sync::Mutex;

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_endpoint_preserves_live_listeners_files_and_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let directory = std::env::temp_dir().join(format!("freergb-ipc-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("api.sock");
        let first = bind_socket(&path).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            bind_socket(&path).unwrap_err().kind(),
            std::io::ErrorKind::AddrInUse
        );
        assert!(tokio::net::UnixStream::connect(&path).await.is_ok());
        drop(first);
        let recovered = bind_socket(&path).unwrap();
        drop(recovered);
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"preserve").unwrap();
        assert!(bind_socket(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"preserve");
        std::fs::remove_file(&path).unwrap();
        let target = directory.join("target");
        std::fs::write(&target, b"preserve").unwrap();
        symlink(&target, &path).unwrap();
        assert!(bind_socket(&path).is_err());
        assert!(std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&target).unwrap(), b"preserve");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn oversized_unterminated_request_closes_connection() {
        let engine = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::RED },
            30,
        )));
        let (client, server) = tokio::io::duplex(4096);
        let task = tokio::spawn(serve_conn(server, engine, Arc::new(Notify::new())));
        let (read, mut write) = tokio::io::split(client);
        let send =
            tokio::spawn(async move { write.write_all(&vec![b'x'; MAX_REQUEST_BYTES + 1]).await });
        let mut lines = BufReader::new(read).lines();
        let response = tokio::time::timeout(std::time::Duration::from_secs(5), lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response).unwrap()["error"]["code"],
            -32600
        );
        assert!(lines.next_line().await.unwrap().is_none());
        task.await.unwrap();
        let _ = send.await.unwrap();
    }

    #[tokio::test]
    async fn exact_limit_request_is_accepted() {
        let engine = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::RED },
            30,
        )));
        let (client, server) = tokio::io::duplex(4096);
        let task = tokio::spawn(serve_conn(server, engine, Arc::new(Notify::new())));
        let (read, mut write) = tokio::io::split(client);
        let mut bytes = br#"{"id":1,"method":"daemon.version"}"#.to_vec();
        bytes.resize(MAX_REQUEST_BYTES - 1, b' ');
        bytes.push(b'\n');
        let send = tokio::spawn(async move {
            write.write_all(&bytes).await.unwrap();
            write.shutdown().await.unwrap();
        });
        let mut lines = BufReader::new(read).lines();
        let response = tokio::time::timeout(std::time::Duration::from_secs(5), lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response).unwrap()["result"]["api_version"],
            1
        );
        send.await.unwrap();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn line_protocol_over_duplex_stream() {
        let engine: SharedEngine = Arc::new(Mutex::new(Engine::new(
            EffectSpec::Solid { color: Rgb::RED },
            30,
        )));
        let shutdown = Arc::new(Notify::new());
        let (client, server) = tokio::io::duplex(4096);
        tokio::spawn(serve_conn(server, engine, shutdown.clone()));

        let (r, mut w) = tokio::io::split(client);
        let mut lines = BufReader::new(r).lines();
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"daemon.version\"}\n")
            .await
            .unwrap();
        let resp: serde_json::Value =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(resp["id"], 7);
        assert_eq!(resp["result"]["api_version"], api::API_VERSION);

        w.write_all(b"{\"id\":8,\"method\":\"daemon.shutdown\"}\n")
            .await
            .unwrap();
        lines.next_line().await.unwrap().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), shutdown.notified())
            .await
            .expect("shutdown notified");
    }
}
