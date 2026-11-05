//! Poll verified package files off the render thread; commit at frame boundaries.
use crate::{
    marketplace::Manager,
    process::{HostCommand, ProcessEffect},
    runtime::Fault,
};
use freergb_core::Canvas;
use freergb_effects::Effect;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{mpsc, Arc},
    time::Duration,
};

pub struct ManagedEffect {
    manager: Arc<Manager>,
    id: String,
    current: ProcessEffect,
    previous: Option<ProcessEffect>,
    candidates: mpsc::Receiver<Result<ProcessEffect, String>>,
    stop: mpsc::Sender<()>,
    fault: Fault,
}
impl ManagedEffect {
    pub fn load(
        manager: Arc<Manager>,
        id: &str,
        version: &str,
        parameters: &BTreeMap<String, f64>,
        fault: Fault,
    ) -> Result<Self, String> {
        Self::load_with_host(
            manager,
            id,
            version,
            parameters,
            fault,
            HostCommand::sibling().map_err(|e| e.to_string())?,
        )
    }
    /// Startup-only trusted executable override for process fixtures; never RPC input.
    pub fn load_with_host(
        manager: Arc<Manager>,
        id: &str,
        version: &str,
        parameters: &BTreeMap<String, f64>,
        fault: Fault,
        host: HostCommand,
    ) -> Result<Self, String> {
        let (bytes, values) = manager.package(id, version, parameters)?;
        let current = ProcessEffect::load(&host, &bytes, &values)?;
        let mut fingerprint = Sha256::digest(&bytes);
        let (sender, candidates) = mpsc::sync_channel(1);
        let (stop, stopping) = mpsc::channel();
        let (monitor, monitor_id, version, parameters) = (
            manager.clone(),
            id.to_owned(),
            version.to_owned(),
            parameters.clone(),
        );
        std::thread::Builder::new()
            .name("effect-reload".into())
            .spawn(move || {
                while matches!(
                    stopping.recv_timeout(Duration::from_millis(500)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    if !monitor.enabled(&monitor_id) {
                        break;
                    }
                    let candidate = monitor.package(&monitor_id, &version, &parameters).map(
                        |(bytes, values)| {
                            let next = Sha256::digest(&bytes);
                            if next == fingerprint {
                                return None;
                            }
                            Some((next, ProcessEffect::load(&host, &bytes, &values)))
                        },
                    );
                    let (next, result) = match candidate {
                        Ok(None) => continue,
                        Ok(Some((next, result))) => (Some(next), result),
                        Err(error) => (None, Err(error)),
                    };
                    // Never block shutdown on a render loop that has stopped consuming.
                    match sender.try_send(result) {
                        Ok(()) => {
                            if let Some(next) = next {
                                fingerprint = next;
                            }
                        }
                        Err(mpsc::TrySendError::Full(_)) => {}
                        Err(mpsc::TrySendError::Disconnected(_)) => break,
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            manager,
            id: id.into(),
            current,
            previous: None,
            candidates,
            stop,
            fault,
        })
    }
}
impl Effect for ManagedEffect {
    fn name(&self) -> &str {
        "Marketplace"
    }
    fn render(&mut self, time: Duration, canvas: &mut Canvas) {
        let result = if !self.manager.enabled(&self.id) {
            Err("Active effect was disabled or uninstalled".into())
        } else {
            if let Ok(candidate) = self.candidates.try_recv() {
                match candidate {
                    Ok(effect) => {
                        self.previous = Some(std::mem::replace(&mut self.current, effect));
                    }
                    Err(error) => self
                        .manager
                        .reload_error(format!("Effect reload rejected: {error}")),
                }
            }
            match self.current.frame(time, canvas) {
                Ok(()) => Ok(()),
                Err(error) => {
                    if let Some(mut previous) = self.previous.take() {
                        match previous.frame(time, canvas) {
                            Ok(()) => {
                                self.current = previous;
                                self.manager
                                    .reload_error(format!("Effect rolled back: {error}"));
                                Ok(())
                            }
                            Err(previous_error) => {
                                Err(format!("{error}; previous effect failed: {previous_error}"))
                            }
                        }
                    } else {
                        Err(error)
                    }
                }
            }
        };
        if let Err(error) = result {
            *self.fault.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
        }
    }
}
impl Drop for ManagedEffect {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}
