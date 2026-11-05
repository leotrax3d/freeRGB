//! Read-only evdev input. No grab, text translation, event persistence or writes.
//! Decoder fixtures run on every OS; native I/O is Linux-only.

#[derive(Default)]
struct Decoder {
    dropped: bool,
}
impl Decoder {
    fn age_ms(record: &[u8], time_bytes: usize, now_ms: u64) -> Option<u64> {
        let (seconds, micros) = match time_bytes {
            8 if record.len() == 16 => (
                i64::from(i32::from_ne_bytes(record[..4].try_into().ok()?)),
                i64::from(i32::from_ne_bytes(record[4..8].try_into().ok()?)),
            ),
            16 if record.len() == 24 => (
                i64::from_ne_bytes(record[..8].try_into().ok()?),
                i64::from_ne_bytes(record[8..16].try_into().ok()?),
            ),
            _ => return None,
        };
        if seconds < 0 || !(0..1_000_000).contains(&micros) {
            return None;
        }
        let at = (seconds as u64)
            .checked_mul(1000)?
            .checked_add(micros as u64 / 1000)?;
        now_ms.checked_sub(at).filter(|age| *age <= 250)
    }
    fn event(&mut self, kind: u16, code: u16, value: i32) -> Option<(u16, bool)> {
        if kind == 0 {
            if code == 3 {
                self.dropped = true;
            } // SYN_DROPPED
            if code == 0 {
                self.dropped = false;
            } // SYN_REPORT
            return None;
        }
        if self.dropped || kind != 1 || !matches!(value, 0 | 1) {
            return None;
        }
        let (scan, extended) = match code {
            1..=83 | 86..=88 => (code, false),
            96 => (0x1c, true),
            97 => (0x1d, true),
            98 => (0x35, true),
            99 => (0x37, true),
            100 => (0x38, true),
            102 => (0x47, true),
            103 => (0x48, true),
            104 => (0x49, true),
            105 => (0x4b, true),
            106 => (0x4d, true),
            107 => (0x4f, true),
            108 => (0x50, true),
            109 => (0x51, true),
            110 => (0x52, true),
            111 => (0x53, true),
            119 => return Some((72, value == 1)),
            125 => (0x5b, true),
            126 => (0x5c, true),
            127 => (0x5d, true),
            _ => return None,
        };
        freergb_effects::keyboard::scan_to_usage(scan, extended).map(|usage| (usage, value == 1))
    }
    fn record(&mut self, record: &[u8], time_bytes: usize) -> Option<(u16, bool)> {
        if !matches!(time_bytes, 8 | 16) || record.len() != time_bytes + 8 {
            return None;
        }
        self.event(
            u16::from_ne_bytes(record[time_bytes..time_bytes + 2].try_into().ok()?),
            u16::from_ne_bytes(record[time_bytes + 2..time_bytes + 4].try_into().ok()?),
            i32::from_ne_bytes(record[time_bytes + 4..].try_into().ok()?),
        )
    }
}

#[cfg(target_os = "linux")]
pub(super) fn run(
    stop: &std::sync::atomic::AtomicBool,
    status: &std::sync::Mutex<(String, Option<String>)>,
    sender: std::sync::mpsc::SyncSender<(super::InputEvent, std::time::Instant)>,
) -> Result<(), String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    struct Input {
        path: std::path::PathBuf,
        file: std::fs::File,
        decoder: Decoder,
        pending: Vec<u8>,
    }
    let mut inputs: Vec<Input> = Vec::new();
    let mut scan_at = Instant::now();
    // Linux input_event uses two native longs followed by type/code/value.
    let time_bytes = 2 * std::mem::size_of::<isize>();
    let record_size = time_bytes + 8;
    while !stop.load(Ordering::SeqCst) {
        let mut denied = false;
        if Instant::now() >= scan_at {
            scan_at = Instant::now() + Duration::from_secs(1);
            for directory in ["/dev/input/by-id", "/dev/input/by-path"] {
                if let Ok(entries) = std::fs::read_dir(directory) {
                    for entry in entries.flatten().take(256) {
                        if !entry.file_name().to_string_lossy().ends_with("-event-kbd") {
                            continue;
                        }
                        let Ok(path) = entry.path().canonicalize() else {
                            continue;
                        };
                        if inputs.iter().any(|i| i.path == path) {
                            continue;
                        }
                        match std::fs::OpenOptions::new()
                            .read(true)
                            .custom_flags(0x800)
                            .open(&path)
                        {
                            Ok(file) => inputs.push(Input {
                                path,
                                file,
                                decoder: Decoder::default(),
                                pending: Vec::new(),
                            }),
                            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                                denied = true
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
            let state = if inputs.is_empty() {
                "waiting_for_keyboard"
            } else {
                "listening"
            };
            *status.lock().unwrap_or_else(|e| e.into_inner()) = (
                state.into(),
                denied.then(|| {
                    "evdev permission denied; see Linux input permissions in INSTALL.md".into()
                }),
            );
        }
        inputs.retain_mut(|input| {
            let mut bytes = [0u8; 768];
            // Bound work per device per poll; release/repeat events never become pulses.
            match input.file.read(&mut bytes) {
                Ok(0) => false,
                Ok(n) => {
                    input.pending.extend_from_slice(&bytes[..n]);
                    let complete = input.pending.len() / record_size * record_size;
                    for record in input.pending[..complete].chunks_exact(record_size) {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                            .min(u128::from(u64::MAX)) as u64;
                        // Decode SYN_DROPPED even when its timestamp is stale.
                        if let Some((usage, pressed)) = input.decoder.record(record, time_bytes) {
                            if let Some(age) = Decoder::age_ms(record, time_bytes, now) {
                                let event = super::InputEvent {
                                    usage,
                                    age_ms: age,
                                    pressed,
                                    device_id: input
                                        .path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned()),
                                };
                                let _ = sender.try_send((event, Instant::now()));
                            }
                        }
                    }
                    input.pending.drain(..complete);
                    true
                }
                Err(e) => e.kind() == std::io::ErrorKind::WouldBlock,
            }
        });
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_physical_presses_and_no_repeat_release_mouse_or_dropped_events() {
        let mut decoder = Decoder::default();
        for (code, usage) in [
            (30, 4),
            (21, 28),
            (44, 29),
            (86, 100),
            (96, 88),
            (97, 228),
            (103, 82),
            (119, 72),
        ] {
            assert_eq!(decoder.event(1, code, 1), Some((usage, true)));
            assert_eq!(decoder.event(1, code, 0), Some((usage, false)));
            assert_eq!(decoder.event(1, code, 2), None);
        }
        assert_eq!(decoder.event(1, 272, 1), None);
        assert_eq!(decoder.event(2, 30, 1), None);
        decoder.event(0, 3, 0);
        assert_eq!(decoder.event(1, 30, 1), None);
        decoder.event(0, 0, 0);
        assert_eq!(decoder.event(1, 30, 1), Some((4, true)));
    }
    #[test]
    fn native_32_and_64_bit_records_are_checked() {
        for time_bytes in [8, 16] {
            let mut record = vec![0; time_bytes];
            record.extend(1u16.to_ne_bytes());
            record.extend(30u16.to_ne_bytes());
            record.extend(1i32.to_ne_bytes());
            assert_eq!(
                Decoder::default().record(&record, time_bytes),
                Some((4, true))
            );
            assert_eq!(
                Decoder::default().record(&record[..record.len() - 1], time_bytes),
                None
            );
        }
        assert_eq!(Decoder::default().record(&[0; 8], 0), None);
    }
    #[test]
    fn queued_future_and_invalid_timestamps_never_replay() {
        for time_bytes in [8, 16] {
            let mut record = if time_bytes == 8 {
                [10i32.to_ne_bytes(), 123_000i32.to_ne_bytes()].concat()
            } else {
                [10i64.to_ne_bytes(), 123_000i64.to_ne_bytes()].concat()
            };
            record.extend([0; 8]);
            assert_eq!(Decoder::age_ms(&record, time_bytes, 10_223), Some(100));
            assert_eq!(Decoder::age_ms(&record, time_bytes, 10_374), None);
            assert_eq!(Decoder::age_ms(&record, time_bytes, 10_122), None);
            record[..time_bytes].fill(255);
            assert_eq!(Decoder::age_ms(&record, time_bytes, 10_223), None);
        }
    }
}
