//! Pure trusted-sampler policy. A fallback decision is not a hardware write.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryFault {
    Clock,
    Identity,
    Invalid,
    Stale,
    Critical,
    Stalled,
}

#[derive(Clone, Copy, Debug)]
pub struct Reading<'a> {
    pub source: &'a str,
    /// Acquisition time in the supervisor's monotonic clock domain.
    pub acquired_ms: u64,
    pub value: f32,
}

pub struct TelemetryGuard {
    temperature_source: String,
    rpm_source: String,
    critical_c: f32,
    last_now: u64,
    temperature: Option<(u64, f32)>,
    rpm: Option<(u64, f32)>,
    stalled_since: Option<u64>,
    fault: Option<TelemetryFault>,
}

impl TelemetryGuard {
    pub fn new(
        temperature_source: String,
        rpm_source: String,
        critical_c: f32,
    ) -> Result<Self, String> {
        if [&temperature_source, &rpm_source]
            .iter()
            .any(|id| id.trim().is_empty() || id.len() > 512 || id.chars().any(char::is_control))
            || temperature_source == rpm_source
            || !(40.0..=100.0).contains(&critical_c)
        {
            return Err(
                "Distinct sensor identities and critical temperature 40..100 C required".into(),
            );
        }
        Ok(Self {
            temperature_source,
            rpm_source,
            critical_c,
            last_now: 0,
            temperature: None,
            rpm: None,
            stalled_since: None,
            fault: None,
        })
    }

    fn latch(&mut self, fault: TelemetryFault) -> Result<(), TelemetryFault> {
        Err(*self.fault.get_or_insert(fault))
    }

    /// Each source may update separately. Repeated acquisition times are faults;
    /// cached readings must not be republished as newly acquired samples.
    pub fn observe(&mut self, reading: Reading<'_>, now_ms: u64) -> Result<(), TelemetryFault> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        if now_ms < self.last_now || reading.acquired_ms > now_ms {
            return self.latch(TelemetryFault::Clock);
        }
        self.last_now = now_ms;
        let temperature = reading.source == self.temperature_source;
        if !temperature && reading.source != self.rpm_source {
            return self.latch(TelemetryFault::Identity);
        }
        let previous = if temperature {
            self.temperature
        } else {
            self.rpm
        };
        if previous.is_some_and(|(at, _)| reading.acquired_ms <= at) {
            return self.latch(TelemetryFault::Clock);
        }
        let range = if temperature {
            -20.0..=150.0
        } else {
            0.0..=100_000.0
        };
        if !range.contains(&reading.value) {
            return self.latch(TelemetryFault::Invalid);
        }
        if now_ms - reading.acquired_ms > 1500 {
            return self.latch(TelemetryFault::Stale);
        }
        if temperature {
            self.temperature = Some((reading.acquired_ms, reading.value));
            if reading.value >= self.critical_c {
                return self.latch(TelemetryFault::Critical);
            }
        } else {
            self.rpm = Some((reading.acquired_ms, reading.value));
            if reading.value < 100.0 {
                let since = *self.stalled_since.get_or_insert(reading.acquired_ms);
                if reading.acquired_ms - since >= 1000 {
                    return self.latch(TelemetryFault::Stalled);
                }
            } else {
                self.stalled_since = None;
            }
        }
        Ok(())
    }

    /// Call independently of command receipt. Missing startup readings refuse
    /// control but permit the sampler to complete initial collection.
    pub fn check(&mut self, now_ms: u64) -> Result<(), TelemetryFault> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        if now_ms < self.last_now {
            return self.latch(TelemetryFault::Clock);
        }
        self.last_now = now_ms;
        let (Some((temp_at, _)), Some((rpm_at, _))) = (self.temperature, self.rpm) else {
            return Err(TelemetryFault::Stale);
        };
        if now_ms - temp_at > 1500 || now_ms - rpm_at > 1500 {
            return self.latch(TelemetryFault::Stale);
        }
        if self
            .stalled_since
            .is_some_and(|since| now_ms - since >= 1000)
        {
            return self.latch(TelemetryFault::Stalled);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn guard() -> TelemetryGuard {
        TelemetryGuard::new("cpu".into(), "fan".into(), 80.0).unwrap()
    }
    fn sample(
        g: &mut TelemetryGuard,
        source: &str,
        at: u64,
        value: f32,
    ) -> Result<(), TelemetryFault> {
        g.observe(
            Reading {
                source,
                acquired_ms: at,
                value,
            },
            at,
        )
    }
    #[test]
    fn sources_age_independently_and_faults_cannot_revive() {
        let mut g = guard();
        assert_eq!(g.check(0), Err(TelemetryFault::Stale));
        sample(&mut g, "cpu", 0, 40.0).unwrap();
        sample(&mut g, "fan", 0, 900.0).unwrap();
        sample(&mut g, "fan", 1500, 900.0).unwrap();
        assert_eq!(g.check(1500), Ok(()));
        assert_eq!(g.check(1501), Err(TelemetryFault::Stale));
        assert_eq!(
            sample(&mut g, "cpu", 1502, 40.0),
            Err(TelemetryFault::Stale)
        );
    }
    #[test]
    fn stall_and_transient_heat_latch_before_next_poll() {
        let mut g = guard();
        sample(&mut g, "cpu", 0, 40.0).unwrap();
        sample(&mut g, "fan", 0, 0.0).unwrap();
        assert_eq!(g.check(999), Ok(()));
        assert_eq!(g.check(1000), Err(TelemetryFault::Stalled));
        let mut g = guard();
        assert_eq!(
            sample(&mut g, "cpu", 0, 80.0),
            Err(TelemetryFault::Critical)
        );
        assert_eq!(
            sample(&mut g, "cpu", 1, 40.0),
            Err(TelemetryFault::Critical)
        );
    }
    #[test]
    fn identity_invalid_time_and_cached_readings_reject() {
        let mut g = guard();
        assert_eq!(
            sample(&mut g, "other", 0, 40.0),
            Err(TelemetryFault::Identity)
        );
        for value in [f32::NAN, f32::INFINITY, -21.0, 151.0] {
            assert_eq!(
                sample(&mut guard(), "cpu", 0, value),
                Err(TelemetryFault::Invalid)
            );
        }
        let mut g = guard();
        sample(&mut g, "cpu", 1, 40.0).unwrap();
        assert_eq!(sample(&mut g, "cpu", 1, 40.0), Err(TelemetryFault::Clock));
        assert_eq!(
            guard().observe(
                Reading {
                    source: "cpu",
                    acquired_ms: 2,
                    value: 40.0
                },
                1
            ),
            Err(TelemetryFault::Clock)
        );
        assert!(TelemetryGuard::new("a".into(), "a".into(), 80.0).is_err());
    }
}
