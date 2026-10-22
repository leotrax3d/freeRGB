//! Daemon-owned live cooling jobs. GUI only supplies explicit selection and user
//! intent; the protected service authorizes/binds native sources and checks IO.
use crate::{
    cooling_host::CoolingHost,
    cooling_runtime::{ControlMode, RuntimeReport},
    native_bridge,
};
use freergb_control::FanCurve;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    pub mode: String,
    pub control_id: String,
    pub temperature_id: String,
    pub rpm_id: String,
    pub confirmed_fan: bool,
    pub curve: Option<FanCurve>,
    pub critical_c: Option<f32>,
}
impl StartRequest {
    fn validate(&self) -> Result<(ControlMode, u8, u8), String> {
        if !self.confirmed_fan {
            return Err("Explicit physical fan confirmation required; pumps excluded".into());
        }
        let sensor = |id: &str, kind: &str| -> Option<String> {
            if id.len() > 128
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'/' || c == b'_')
            {
                return None;
            }
            let (parent, index) = id.rsplit_once('/')?;
            let index = index.parse::<u8>().ok()?;
            let base = parent.strip_suffix(&format!("/{kind}"))?;
            let parts: Vec<_> = base.split('/').collect();
            if parts.len() != 4
                || !parts[0].is_empty()
                || parts[1] != "lpc"
                || parts[2].is_empty()
                || parts[3].parse::<u16>().is_err()
                || (kind != "temperature" && index > 6)
            {
                return None;
            }
            Some(base.into())
        };
        let base = sensor(&self.control_id, "control").ok_or("Invalid native control identity")?;
        if sensor(&self.temperature_id, "temperature").as_ref() != Some(&base)
            || sensor(&self.rpm_id, "fan").as_ref() != Some(&base)
        {
            return Err("Exact temperature/RPM on the same native hardware required".into());
        }
        match self.mode.as_str() {
            "curve" if self.critical_c.is_none() => {
                let curve = self.curve.clone().ok_or("Curve required")?;
                curve.validate()?;
                if curve.minimum_percent < 40.0 || curve.critical_c > 100.0 {
                    return Err("Native curve floor >=40%, critical <=100 C required".into());
                }
                let critical = curve.critical_c.floor() as u8;
                let minimum = curve.minimum_percent.ceil() as u8;
                Ok((ControlMode::Curve(curve), critical, minimum))
            }
            "calibration" if self.curve.is_none() => {
                let critical = self.critical_c.unwrap_or(80.0);
                if !(40.0..=100.0).contains(&critical) {
                    return Err("Calibration critical temperature must be 40..100 C".into());
                }
                Ok((
                    ControlMode::Calibration {
                        critical_c: critical,
                    },
                    critical.floor() as u8,
                    40,
                ))
            }
            _ => Err("Expected curve or calibration with unambiguous parameters".into()),
        }
    }
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub available: bool,
    pub job_id: Option<u64>,
    pub state: String,
    pub report: Option<RuntimeReport>,
    pub error: Option<String>,
}
type View = Arc<Mutex<Status>>;
enum Command {
    Start(u64, StartRequest),
    Stop(u64),
}
pub struct CoolingSessions {
    sender: Option<mpsc::Sender<Command>>,
    view: View,
    thread: Option<std::thread::JoinHandle<()>>,
    next: u64,
}
impl Default for CoolingSessions {
    fn default() -> Self {
        Self {
            sender: None,
            view: Arc::new(Mutex::new(Status {
                available: false,
                job_id: None,
                state: "disabled".into(),
                report: None,
                error: None,
            })),
            thread: None,
            next: 0,
        }
    }
}
fn update(view: &View, state: &str, report: Option<RuntimeReport>, error: Option<String>) {
    let mut value = view.lock().unwrap_or_else(|e| e.into_inner());
    value.state = state.into();
    value.report = report;
    value.error = error;
}
impl CoolingSessions {
    pub fn enable(helper: PathBuf) -> Result<Self, String> {
        if !cfg!(windows) {
            return Err("Native cooling requires Windows".into());
        }
        let mut result = Self::default();
        let (tx, rx) = mpsc::channel(8);
        let view = result.view.clone();
        result
            .view
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .available = true;
        update(&result.view, "idle", None, None);
        result.thread = Some(
            std::thread::Builder::new()
                .name("mainboard-cooling".into())
                .spawn(move || {
                    match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(runtime) => runtime.block_on(worker(helper, view, rx)),
                        Err(error) => update(&view, "failed", None, Some(error.to_string())),
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        result.sender = Some(tx);
        Ok(result)
    }
    pub fn status(&self) -> Status {
        self.view.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn start(&mut self, request: StartRequest) -> Result<u64, String> {
        request.validate()?;
        let sender = self
            .sender
            .as_ref()
            .ok_or("Mainboard provider/control not enabled")?;
        let state = self.status();
        if state.state == "blocked" {
            return Err("Native recovery inspection required before daemon restart".into());
        }
        if ["authorizing", "executing", "stopping"].contains(&state.state.as_str()) {
            return Err("A cooling job is already active".into());
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Cooling job identity exhausted")?;
        {
            let mut view = self.view.lock().unwrap_or_else(|e| e.into_inner());
            view.job_id = Some(self.next);
            view.state = "authorizing".into();
            view.report = None;
            view.error = None;
        }
        if sender.try_send(Command::Start(self.next, request)).is_err() {
            update(
                &self.view,
                "failed",
                None,
                Some("Cooling worker unavailable".into()),
            );
            return Err("Cooling worker unavailable".into());
        }
        Ok(self.next)
    }
    pub fn stop(&self, id: u64) -> Result<(), String> {
        let status = self.status();
        if status.job_id != Some(id)
            || !["authorizing", "executing", "stopping"].contains(&status.state.as_str())
        {
            return Err("Cooling job is not active".into());
        }
        self.sender
            .as_ref()
            .ok_or("Cooling disabled")?
            .try_send(Command::Stop(id))
            .map_err(|_| "Cooling queue unavailable".to_owned())
    }
}
impl Drop for CoolingSessions {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn worker(helper: PathBuf, view: View, mut commands: mpsc::Receiver<Command>) {
    while let Some(command) = commands.recv().await {
        let Command::Start(id, request) = command else {
            continue;
        };
        let (mode, critical, minimum) = match request.validate() {
            Ok(v) => v,
            Err(error) => {
                update(&view, "failed", None, Some(error));
                continue;
            }
        };
        // Do not cancel an elevated authorization child: it may still publish
        // authorization after the requesting helper is dropped. Drain a queued
        // stop only after authorization finishes, before native acquisition.
        let authorized = native_bridge::authorize(
            &helper,
            &request.control_id,
            &request.temperature_id,
            &request.rpm_id,
            critical,
            minimum,
        )
        .await;
        match commands.try_recv() {
            Ok(Command::Stop(job)) if job == id => {
                update(&view, "stopped", None, None);
                continue;
            }
            Err(mpsc::error::TryRecvError::Disconnected) => return,
            _ => {}
        }
        if let Err(error) = authorized {
            update(&view, "failed", None, Some(error));
            continue;
        }
        let mut host = CoolingHost::new(true, Instant::now());
        if let Err(error) = host.start_native(native_bridge::connect(helper.clone()), mode) {
            update(&view, "failed", None, Some(error));
            continue;
        }
        update(&view, "executing", None, None);
        let mut ticks = tokio::time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                _=ticks.tick()=>{
                    match host.collect_finished().await {
                        Ok(Some(report))=>{update(&view,"completed",Some(report),None);break;},
                        Ok(None)=>{},
                        Err(error)=>{update(&view,"blocked",None,Some(error));break;},
                    }
                },
                command=commands.recv()=>{
                    let shutting_down=command.is_none();
                    if !shutting_down && !matches!(command,Some(Command::Stop(job)) if job==id){continue;}
                    update(&view,"stopping",None,None);
                    match host.prepare_maintenance(20000).await {
                        Ok(())=>update(&view,"stopped",host.last_report().cloned(),None),
                        Err(error)=>update(&view,"blocked",None,Some(error)),
                    }
                    if shutting_down{return;}break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> StartRequest {
        serde_json::from_value(json!({
            "mode":"calibration", "control_id":"/lpc/nct6701d/0/control/2",
            "rpm_id":"/lpc/nct6701d/0/fan/2", "temperature_id":"/lpc/nct6701d/0/temperature/1",
            "confirmed_fan":true
        }))
        .unwrap()
    }

    #[test]
    fn unsafe_or_ambiguous_native_requests_are_rejected() {
        assert!(request().validate().is_ok());
        let mut r = request();
        r.confirmed_fan = false;
        assert!(r.validate().is_err());
        r = request();
        r.rpm_id = "/lpc/nct6701d/1/fan/2".into();
        assert!(r.validate().is_err());
        r = request();
        r.control_id = "/lpc/nct6701d/0/control/7".into();
        assert!(r.validate().is_err());
        r = request();
        r.critical_c = Some(f32::NAN);
        assert!(r.validate().is_err());
        r = request();
        r.curve = Some(FanCurve::default());
        assert!(r.validate().is_err());
        r.mode = "curve".into();
        assert!(r.validate().is_err()); // default 25% floor cannot reach native IO
        let curve = r.curve.as_mut().unwrap();
        curve.minimum_percent = 40.0;
        curve.points[0].duty_percent = 40.0;
        assert!(r.validate().is_ok());
    }

    #[test]
    fn disabled_and_blocked_jobs_never_queue_acquisition() {
        let mut sessions = CoolingSessions::default();
        assert!(!sessions.status().available);
        assert!(sessions.start(request()).is_err());
        assert!(sessions.stop(1).is_err());
        let (sender, mut receiver) = mpsc::channel(8);
        sessions.sender = Some(sender);
        update(
            &sessions.view,
            "blocked",
            None,
            Some("return unverified".into()),
        );
        assert!(sessions.start(request()).is_err());
        assert!(receiver.try_recv().is_err());
    }
}
