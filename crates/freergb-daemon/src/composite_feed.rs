//! Latest composite-curve outputs, published by the daemon's sensor refresh
//! and read by live cooling sessions in `mode:"composite"`. A missing, stale or
//! fail-safe output always means 100%; the protected worker's own lease and
//! temperature supervision stay in force on top of this.
use freergb_control::curves::CurveOutput;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Older outputs are treated as missing (the refresh runs every second).
pub const MAX_AGE: Duration = Duration::from_millis(3000);

type Outputs = Option<(Instant, BTreeMap<String, CurveOutput>)>;

/// Shared handle: the daemon's `Controls` publishes, sessions read.
#[derive(Clone, Debug, Default)]
pub struct CompositeFeed(Arc<Mutex<Outputs>>);

impl CompositeFeed {
    pub fn publish(&self, outputs: BTreeMap<String, CurveOutput>, at: Instant) {
        if let Ok(mut feed) = self.0.lock() {
            *feed = Some((at, outputs));
        }
    }

    /// Duty for `output` at `now`, or 100% when it cannot be trusted.
    pub fn duty(&self, output: &str, now: Instant) -> f32 {
        let Ok(feed) = self.0.lock() else {
            return 100.0;
        };
        match &*feed {
            Some((at, outputs)) if now.saturating_duration_since(*at) <= MAX_AGE => {
                match outputs.get(output) {
                    Some(o) if !o.failsafe => o.percent.clamp(0.0, 100.0),
                    _ => 100.0,
                }
            }
            _ => 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_stale_and_failsafe_outputs_mean_full_duty() {
        let feed = CompositeFeed::default();
        let t0 = Instant::now();
        assert_eq!(feed.duty("ok", t0), 100.0);
        let ok = CurveOutput {
            percent: 42.0,
            failsafe: false,
        };
        let bad = CurveOutput {
            percent: 100.0,
            failsafe: true,
        };
        feed.publish(
            [("ok".to_string(), ok), ("bad".to_string(), bad)].into(),
            t0,
        );
        assert_eq!(feed.duty("ok", t0), 42.0);
        assert_eq!(feed.duty("bad", t0), 100.0);
        assert_eq!(feed.duty("missing", t0), 100.0);
        assert_eq!(feed.duty("ok", t0 + Duration::from_secs(4)), 100.0);
    }
}
