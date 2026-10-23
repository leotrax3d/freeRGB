//! Bounded process-lifetime history, independent of GUI connections.
use serde::Serialize;
use std::collections::{BTreeSet, VecDeque};
#[derive(Clone, Debug, Serialize)]
pub struct Incident {
    pub id: u64,
    pub source: String,
    pub message: String,
    pub details: String,
}
pub struct Journal {
    pub session: String,
    next: u64,
    active: BTreeSet<String>,
    entries: VecDeque<Incident>,
}
impl Default for Journal {
    fn default() -> Self {
        Self {
            session: format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            next: 0,
            active: Default::default(),
            entries: Default::default(),
        }
    }
}
impl Journal {
    pub fn observe(&mut self, source: &str, message: &str, error: Option<&str>) {
        let Some(details) = error else {
            self.active.remove(source);
            return;
        };
        if !self.active.insert(source.into()) {
            return;
        }
        self.next += 1;
        tracing::warn!(source, error = details, "{message}");
        self.entries.push_back(Incident {
            id: self.next,
            source: source.into(),
            message: message.into(),
            details: details.into(),
        });
        if self.entries.len() > 64 {
            self.entries.pop_front();
        }
    }
    pub fn entries(&self) -> Vec<Incident> {
        self.entries.iter().cloned().collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deduplicates_until_recovery_and_bounds_offline_history() {
        let mut j = Journal::default();
        j.observe("device", "Disconnected", Some("timeout"));
        j.observe("device", "Disconnected", Some("unplugged"));
        assert_eq!(j.entries().len(), 1);
        j.observe("device", "", None);
        j.observe("device", "Disconnected", Some("timeout"));
        assert_eq!(j.entries()[1].id, 2);
        for n in 0..100 {
            j.observe(&format!("device{n}"), "Failed", Some("error"));
        }
        assert_eq!(j.entries().len(), 64);
        assert_eq!(j.entries().last().unwrap().id, 102);
    }
}
