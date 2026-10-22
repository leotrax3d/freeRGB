//! freeRGB desktop GUI backend. Holds no device logic: the webview reaches the
//! daemon only through the commands below, which reuse `freergb-client`.
//! Closing the window exits this process; the daemon keeps running.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod diagnostics;
mod memory;
mod poller;
mod queue;

use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

struct AppState {
    jobs: Mutex<Sender<queue::Job>>,
    /// Wakes the poller, e.g. so a newly opened view gets its data at once.
    wake: Mutex<Sender<()>>,
    shared: Arc<Mutex<poller::Shared>>,
}

/// Calls one allowlisted daemon method through the serialized queue.
#[tauri::command]
async fn rpc(
    method: String,
    params: Option<Value>,
    state: tauri::State<'_, AppState>,
) -> queue::Reply {
    if !queue::ALLOWED.contains(&method.as_str()) {
        return Err(queue::CallError::Forbidden { method });
    }
    let (reply, rx) = std::sync::mpsc::channel();
    let job = queue::Job {
        method,
        params: params.unwrap_or(Value::Null),
        reply,
    };
    let sent = state
        .jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .send(job);
    if sent.is_err() {
        return Err(queue::CallError::Transport {
            message: "command queue stopped".into(),
        });
    }
    tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(Err(queue::CallError::Transport {
            message: "command queue stopped".into(),
        }))
}

/// Latest value of every slice; used once when the frontend loads.
#[tauri::command]
fn snapshot(state: tauri::State<'_, AppState>) -> BTreeMap<String, Value> {
    state
        .shared
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .slices
        .clone()
}

/// Replaces the set of optional slices the visible views need.
#[tauri::command]
fn want_slices(slices: Vec<String>, state: tauri::State<'_, AppState>) {
    let known: Vec<_> = poller::OPTIONAL.iter().map(|(s, _)| *s).collect();
    let mut shared = state.shared.lock().unwrap_or_else(|e| e.into_inner());
    shared.wanted = slices
        .into_iter()
        .filter(|s| known.contains(&s.as_str()))
        .collect();
    // Drop stale optional values so a view never shows data from long ago.
    let wanted = shared.wanted.clone();
    shared
        .slices
        .retain(|slice, _| !known.contains(&slice.as_str()) || wanted.contains(slice));
    drop(shared);
    let _ = state
        .wake
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .send(());
}

/// Fetches the daemon's redacted report and saves it; returns the file path.
#[tauri::command]
async fn export_diagnostics() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let report =
            freergb_client::call("diagnostics.export", Value::Null).map_err(|e| e.to_string())?;
        diagnostics::save(&report).map(|path| path.display().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Starts `freergbd` from the application directory (or `FREERGB_DAEMON`),
/// like `freergb-launcher` does. Never stops a running daemon.
#[tauri::command]
fn start_service() -> Result<(), String> {
    // Installed starts must reuse the launcher's fixed provider configuration.
    if let Some(launcher) = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent().map(|dir| {
                dir.join(if cfg!(windows) {
                    "freergb-launcher.exe"
                } else {
                    "freergb-launcher"
                })
            })
        })
        .filter(|path| path.is_file())
    {
        let mut command = std::process::Command::new(launcher);
        command.arg("--daemon-only");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        return command.spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    let name = if cfg!(windows) {
        "freergbd.exe"
    } else {
        "freergbd"
    };
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)));
    let path = std::env::var_os("FREERGB_DAEMON")
        .map(std::path::PathBuf::from)
        .or(beside)
        .filter(|path| path.is_file())
        .ok_or("freergbd was not found next to the application")?;
    let mut command = std::process::Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command.spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_classic() -> Result<(), String> {
    let path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name(if cfg!(windows) {
            "freergb-gui.exe"
        } else {
            "freergb-gui"
        });
    std::process::Command::new(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn main() {
    let shared = Arc::new(Mutex::new(poller::Shared::default()));
    let (jobs_tx, jobs_rx) = queue::channel();
    let (wake_tx, wake_rx) = std::sync::mpsc::channel::<()>();

    tauri::Builder::default()
        .manage(AppState {
            jobs: Mutex::new(jobs_tx),
            wake: Mutex::new(wake_tx.clone()),
            shared: shared.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            rpc,
            snapshot,
            want_slices,
            start_service,
            open_classic,
            export_diagnostics
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                let minimized = std::sync::atomic::AtomicBool::new(false);
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Resized(_) = event {
                        let now = w.is_minimized().unwrap_or(false);
                        if minimized.swap(now, std::sync::atomic::Ordering::Relaxed) != now {
                            memory::set_minimized(&w, now);
                        }
                    }
                });
            }
            std::thread::Builder::new()
                .name("daemon-commands".into())
                .spawn(move || {
                    queue::run(
                        jobs_rx,
                        |method, params| Ok(freergb_client::call(method, params)?),
                        || {
                            let _ = wake_tx.send(());
                        },
                    )
                })?;
            std::thread::Builder::new()
                .name("daemon-poller".into())
                .spawn(move || {
                    let window = handle.get_webview_window("main");
                    poller::run(
                        shared,
                        wake_rx,
                        poller::Deps {
                            call: |method: &str| freergb_client::call(method, Value::Null),
                            emit: |slice: &str, value: &Value| {
                                let _ =
                                    handle.emit(poller::EVENT, poller::SliceEvent { slice, value });
                            },
                            hidden: || {
                                window.as_ref().is_some_and(|w| {
                                    w.is_minimized().unwrap_or(false)
                                        || !w.is_visible().unwrap_or(true)
                                })
                            },
                        },
                    )
                })?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running freeRGB");
}
