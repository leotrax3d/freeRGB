//! Isolate xcap's portal/PipeWire loop in a child. Killing/reaping the child
//! closes portal sessions and streams even if the upstream loop is blocked.
use crate::{ScreenError, ScreenFrame, FRAME_H, FRAME_W};
const FRAME_BYTES: usize = FRAME_W * FRAME_H * 4;
const HEADER: &[u8; 8] = b"FRGB\x01\0\0\0";

#[cfg(target_os = "linux")]
fn wayland_session() -> bool {
    // Match xcap's backend detection, including sessions that only export XDG.
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
        || std::env::var("WAYLAND_DISPLAY").is_ok_and(|v| v.to_lowercase().contains("wayland"))
}

fn decode(bytes: &[u8]) -> Result<ScreenFrame, ScreenError> {
    if bytes.len() != FRAME_BYTES {
        return Err(ScreenError::Capture("Invalid capture helper frame".into()));
    }
    Ok(ScreenFrame::downscale_rgba(
        bytes, FRAME_W, FRAME_H, FRAME_W, FRAME_H,
    ))
}

#[cfg(target_os = "linux")]
pub(super) fn start(fps: u32) -> Result<crate::ScreenInput, ScreenError> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    };
    use std::time::Duration;
    let executable = std::env::current_exe().map_err(|e| ScreenError::Capture(e.to_string()))?;
    let helper = executable
        .parent()
        .ok_or(ScreenError::NoMonitor)?
        .join("freergb-screen-capture");
    let mut child = Command::new(helper)
        .arg(fps.clamp(1, 60).to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            ScreenError::Capture(format!(
                "Install freergb-screen-capture alongside freergbd: {e}"
            ))
        })?;
    let mut output = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut header = [0; 8];
        if output.read_exact(&mut header).is_err() || &header != HEADER {
            return;
        }
        let mut bytes = vec![0; FRAME_BYTES];
        while output.read_exact(&mut bytes).is_ok() {
            match tx.try_send(bytes.clone()) {
                Ok(()) | Err(mpsc::TrySendError::Full(_)) => {}
                Err(mpsc::TrySendError::Disconnected(_)) => break,
            }
        }
    });
    // Portal consent must never hold the daemon render/API mutex. Publish a
    // dark frame while waiting, and expose progress/failure via monitor_name().
    let frame = Arc::new(Mutex::new(ScreenFrame::default()));
    let stop = Arc::new(AtomicBool::new(false));
    let capture_state = Arc::new(std::sync::atomic::AtomicU8::new(0));
    let state = capture_state.clone();
    let (f, s) = (frame.clone(), stop.clone());
    let handle = std::thread::spawn(move || {
        let mut last = std::time::Instant::now();
        while !s.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(bytes) => {
                    last = std::time::Instant::now();
                    if let Ok(value) = decode(&bytes) {
                        *f.lock().unwrap_or_else(|e| e.into_inner()) = value;
                        state.store(1, Ordering::SeqCst);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout)
                    if last.elapsed()
                        < Duration::from_secs(if state.load(Ordering::SeqCst) == 0 {
                            60
                        } else {
                            10
                        }) => {}
                Err(_) => break,
            }
        }
        *f.lock().unwrap_or_else(|e| e.into_inner()) = ScreenFrame::default();
        state.store(2, Ordering::SeqCst);
        let _ = child.kill();
        let _ = child.wait();
        drop(rx);
        let _ = reader.join();
    });
    Ok(crate::ScreenInput {
        frame,
        stop,
        handle: Some(handle),
        capture_state,
        monitor_name: if wayland_session() {
            "Wayland ScreenCast / PipeWire"
        } else {
            "X11"
        }
        .into(),
    })
}

/// Internal helper entry point. stdout is exclusively a fixed-size RGBA protocol.
#[cfg(target_os = "linux")]
pub fn run_helper(fps: u32) -> Result<(), ScreenError> {
    use std::io::Write;
    use std::time::{Duration, Instant};
    let monitors = xcap::Monitor::all().map_err(|e| ScreenError::Capture(e.to_string()))?;
    let monitor = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or(monitors.first())
        .ok_or(ScreenError::NoMonitor)?;
    let mut output = std::io::stdout().lock();
    let period = Duration::from_secs_f64(1.0 / f64::from(fps.clamp(1, 60)));
    output
        .write_all(HEADER)
        .map_err(|e| ScreenError::Capture(e.to_string()))?;
    if wayland_session() {
        // video_recorder uses ScreenCast + PipeWire, unlike capture_image's
        // screenshot portal (which would ask repeatedly and create PNG files).
        let (recorder, receiver) = monitor
            .video_recorder()
            .map_err(|e| ScreenError::Capture(e.to_string()))?;
        recorder
            .start()
            .map_err(|e| ScreenError::Capture(e.to_string()))?;
        let mut last = Instant::now() - period;
        while let Ok(mut frame) = receiver.recv_timeout(Duration::from_secs(10)) {
            // Prefer fresh frames if downscaling or the pipe briefly fell behind.
            for _ in 0..8 {
                match receiver.try_recv() {
                    Ok(newer) => frame = newer,
                    Err(_) => break,
                }
            }
            if last.elapsed() < period {
                continue;
            }
            write_frame(
                &mut output,
                &frame.raw,
                frame.width as usize,
                frame.height as usize,
            )?;
            last = Instant::now();
        }
        let _ = recorder.stop();
    } else {
        loop {
            let started = Instant::now();
            let image = monitor
                .capture_image()
                .map_err(|e| ScreenError::Capture(e.to_string()))?;
            write_frame(
                &mut output,
                image.as_raw(),
                image.width() as usize,
                image.height() as usize,
            )?;
            if let Some(rest) = period.checked_sub(started.elapsed()) {
                std::thread::sleep(rest);
            }
        }
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn write_frame(
    output: &mut impl std::io::Write,
    raw: &[u8],
    w: usize,
    h: usize,
) -> Result<(), ScreenError> {
    if w == 0
        || h == 0
        || w > 16384
        || h > 16384
        || w.checked_mul(h).and_then(|n| n.checked_mul(4)) != Some(raw.len())
    {
        return Err(ScreenError::Capture(
            "Invalid RGBA capture dimensions/stride".into(),
        ));
    }
    let small = ScreenFrame::downscale_rgba(raw, w, h, FRAME_W, FRAME_H);
    for color in small.pixels {
        output
            .write_all(&[color.r, color.g, color.b, 255])
            .map_err(|e| ScreenError::Capture(e.to_string()))?;
    }
    output
        .flush()
        .map_err(|e| ScreenError::Capture(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_frame_protocol_rejects_truncation_and_preserves_channels() {
        for size in [0, FRAME_BYTES - 1, FRAME_BYTES + 1] {
            assert!(decode(&vec![0; size]).is_err());
        }
        let bytes = [12, 34, 56, 255].repeat(FRAME_W * FRAME_H);
        let frame = decode(&bytes).unwrap();
        assert!(frame
            .pixels
            .iter()
            .all(|c| *c == freergb_core::Rgb::new(12, 34, 56)));
        assert_eq!(HEADER, b"FRGB\x01\0\0\0");
    }
    #[test]
    fn helper_frames_validate_dimensions_and_use_the_wire_format() {
        let mut output = Vec::new();
        for (w, h, raw) in [(0, 1, &[][..]), (1, 1, &[1, 2, 3][..]), (16385, 1, &[][..])] {
            assert!(write_frame(&mut output, raw, w, h).is_err());
            assert!(output.is_empty());
        }
        write_frame(&mut output, &[12, 34, 56, 255], 1, 1).unwrap();
        assert_eq!(output.len(), FRAME_BYTES);
        assert_eq!(
            decode(&output).unwrap().pixels[0],
            freergb_core::Rgb::new(12, 34, 56)
        );
    }
}
