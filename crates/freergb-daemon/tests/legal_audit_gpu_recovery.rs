//! Audit regressions (P0.1): synthetic GPU only. No NVML, driver or hardware
//! calls. Derived from docs/legal/repro/gpu_recovery.rs.
use freergb_daemon::gpu_cooling::{GpuAssignment, GpuBackend, GpuCooling, GpuFan, GpuMode};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

const FAN: &str = "simulated:audit-recovery/fan0";

#[derive(Default)]
struct Trace {
    acquisitions: usize,
    fail_release: bool,
}

struct MockGpu(Arc<Mutex<Trace>>);
impl GpuBackend for MockGpu {
    fn name(&self) -> &'static str {
        "audit-mock"
    }
    fn discover(&mut self) -> Vec<GpuFan> {
        vec![GpuFan {
            id: FAN.into(),
            vendor: "simulated",
            gpu_name: "Audit mock".into(),
            index: 0,
            duty_percent: Some(100.0),
            controllable: true,
            reason: None,
            min_percent: 40.0,
        }]
    }
    fn take_control(&mut self, _: &str) -> Result<(), String> {
        self.0.lock().unwrap().acquisitions += 1;
        Ok(())
    }
    fn set_duty(&mut self, _: &str, _: f32) -> Result<(), String> {
        Ok(())
    }
    fn release(&mut self, _: &str) -> Result<(), String> {
        if self.0.lock().unwrap().fail_release {
            Err("synthetic return failure".into())
        } else {
            Ok(())
        }
    }
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "freergb-audit-recovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        // No overwrite/delete of a pre-existing path, even after a prior crash.
        std::fs::create_dir(&path).expect("unique audit directory");
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn assignment() -> GpuAssignment {
    GpuAssignment {
        fan_id: FAN.into(),
        mode: GpuMode::Manual { percent: 50.0 },
    }
}

#[test]
fn failed_return_retains_durable_recovery_entry() {
    let scratch = Scratch::new();
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut gpu = GpuCooling::new(
        vec![Box::new(MockGpu(trace.clone()))],
        Some(scratch.0.clone()),
    );
    gpu.start();
    gpu.assign(assignment()).unwrap();
    trace.lock().unwrap().fail_release = true;
    assert!(gpu.release(Some(FAN)).is_err());
    let journal: serde_json::Value = serde_json::from_slice(
        &std::fs::read(scratch.0.join("gpu-control-journal.json"))
            .expect("recovery journal retained"),
    )
    .unwrap();
    assert!(
        journal["fans"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some(FAN)),
        "a failed return must remain journaled until successful recovery"
    );
}

#[test]
fn unavailable_journal_blocks_take_control() {
    let scratch = Scratch::new();
    let trace = Arc::new(Mutex::new(Trace::default()));
    // Deterministic failure even when tests run as root: state_dir is a file.
    let unavailable = scratch.0.join("not-a-directory");
    std::fs::write(&unavailable, b"synthetic journal failure").unwrap();
    let mut gpu = GpuCooling::new(vec![Box::new(MockGpu(trace.clone()))], Some(unavailable));
    gpu.start();
    let result = gpu.assign(assignment());
    assert_eq!(
        trace.lock().unwrap().acquisitions,
        0,
        "no acquisition without durable recovery intent"
    );
    assert!(result.is_err(), "journal failure must fail assignment");
}

#[test]
fn unresolved_entry_survives_restart_until_release_succeeds() {
    let scratch = Scratch::new();
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut gpu = GpuCooling::new(
        vec![Box::new(MockGpu(trace.clone()))],
        Some(scratch.0.clone()),
    );
    gpu.start();
    gpu.assign(assignment()).unwrap();
    trace.lock().unwrap().fail_release = true;
    std::mem::forget(gpu); // crash with the fan still manual

    // Restart while the driver still refuses the return: entry must stay.
    let mut gpu = GpuCooling::new(
        vec![Box::new(MockGpu(trace.clone()))],
        Some(scratch.0.clone()),
    );
    gpu.start();
    let before = trace.lock().unwrap().acquisitions;
    assert!(
        gpu.assign(assignment()).is_err(),
        "unresolved fan must not be re-acquired"
    );
    assert_eq!(trace.lock().unwrap().acquisitions, before);
    assert!(scratch.0.join("gpu-control-journal.json").exists());
    std::mem::forget(gpu);

    // Driver recovers: next start proves the return and clears the journal.
    trace.lock().unwrap().fail_release = false;
    let gpu = GpuCooling::new(
        vec![Box::new(MockGpu(trace.clone()))],
        Some(scratch.0.clone()),
    );
    let mut gpu = gpu;
    gpu.start();
    assert!(!scratch.0.join("gpu-control-journal.json").exists());
    gpu.assign(assignment()).unwrap();
}

#[test]
fn unreadable_journal_blocks_control_and_is_kept() {
    let scratch = Scratch::new();
    std::fs::write(scratch.0.join("gpu-control-journal.json"), b"{broken").unwrap();
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut gpu = GpuCooling::new(
        vec![Box::new(MockGpu(trace.clone()))],
        Some(scratch.0.clone()),
    );
    gpu.start();
    assert!(gpu.assign(assignment()).is_err());
    assert_eq!(trace.lock().unwrap().acquisitions, 0);
    assert!(scratch.0.join("gpu-control-journal.json.corrupt").exists());
}

#[test]
fn missing_state_dir_blocks_control() {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut gpu = GpuCooling::new(vec![Box::new(MockGpu(trace.clone()))], None);
    gpu.start();
    assert!(gpu.assign(assignment()).is_err());
    assert_eq!(trace.lock().unwrap().acquisitions, 0);
}
