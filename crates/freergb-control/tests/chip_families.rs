//! Chip-family simulators (docs/FAN_COVERAGE.md). Every family in
//! `fixtures/chip_families.json` runs the guided channel test, runtime RPM
//! plausibility, fault return and unrestricted unlock against a register-level
//! fake with the family's quirks. No hardware access: this is simulation only.
use freergb_control::calibration::{Calibration, CalibrationPhase, CalibrationSample};
use freergb_control::channels::ChannelRole;
use freergb_control::fan_support::{limits, ChannelEvidence, SupportTier, SAFE_FAN_FLOOR};
use freergb_control::plausibility::RpmPlausibility;
use freergb_control::telemetry_guard::{Reading, TelemetryFault, TelemetryGuard};
use serde_json::Value;

const FIRMWARE_PERCENT: u8 = 60;
const MAX_RPM: f64 = 1800.0;
const STEP_MS: u64 = 250;

#[derive(Clone, Debug)]
struct Quirks {
    readback: bool,
    raw_max: u16,
    rpm_lag_ms: u64,
    /// Channels whose mode bits share one register byte.
    shared_mode_groups: Vec<Vec<usize>>,
    /// Writes are accepted but the fan keeps following the firmware.
    ignored_channels: Vec<usize>,
    /// The chip clamps lower duties to this value (limited duty range).
    min_effective_percent: u8,
    inverted: bool,
}

#[derive(Clone, Debug)]
struct Family {
    id: String,
    capability: SupportTier,
    controls: usize,
    quirks: Quirks,
}

fn families() -> Vec<Family> {
    let value: Value =
        serde_json::from_str(include_str!("fixtures/chip_families.json")).expect("fixture");
    value["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let q = &f["quirks"];
            let list = |v: &Value| -> Vec<usize> {
                v.as_array()
                    .map(|a| a.iter().map(|i| i.as_u64().unwrap() as usize).collect())
                    .unwrap_or_default()
            };
            Family {
                id: f["id"].as_str().unwrap().to_owned(),
                capability: serde_json::from_value(f["capability"].clone()).unwrap(),
                controls: f["controls"].as_u64().unwrap() as usize,
                quirks: Quirks {
                    readback: q["readback"].as_bool().unwrap(),
                    raw_max: q["raw_max"].as_u64().unwrap() as u16,
                    rpm_lag_ms: q["rpm_lag_ms"].as_u64().unwrap_or(0),
                    shared_mode_groups: q["shared_mode_groups"]
                        .as_array()
                        .map(|g| g.iter().map(list).collect())
                        .unwrap_or_default(),
                    ignored_channels: list(&q["ignored_channels"]),
                    min_effective_percent: q["min_effective_percent"].as_u64().unwrap_or(0) as u8,
                    inverted: q["inverted"].as_bool().unwrap_or(false),
                },
            }
        })
        .collect()
}

/// Register-level fake of one SuperIO chip.
struct SimChip {
    quirks: Quirks,
    /// One mode register per group; bit `i` of `mode[g]` = software mode of
    /// the i-th member. Channels without a shared group get their own register.
    groups: Vec<Vec<usize>>,
    mode: Vec<u8>,
    raw: Vec<u16>,
    rpm: Vec<f64>,
    blocked: Vec<bool>,
    last_ms: u64,
    /// (channel, percent) for every duty write, in order.
    writes: Vec<(usize, u8)>,
}

impl SimChip {
    fn new(family: &Family) -> Self {
        let mut groups = family.quirks.shared_mode_groups.clone();
        for ch in 0..family.controls {
            if !groups.iter().any(|g| g.contains(&ch)) {
                groups.push(vec![ch]);
            }
        }
        let firmware_raw = Self::encode(&family.quirks, FIRMWARE_PERCENT);
        Self {
            quirks: family.quirks.clone(),
            mode: vec![0; groups.len()],
            groups,
            raw: vec![firmware_raw; family.controls],
            rpm: vec![Self::fan_rpm(FIRMWARE_PERCENT); family.controls],
            blocked: vec![false; family.controls],
            last_ms: 0,
            writes: Vec::new(),
        }
    }
    fn encode(q: &Quirks, percent: u8) -> u16 {
        let raw = (u32::from(percent) * u32::from(q.raw_max) + 50) / 100;
        raw as u16
    }
    fn decode(&self, raw: u16) -> u8 {
        let raw = if self.quirks.inverted {
            self.quirks.raw_max - raw
        } else {
            raw
        };
        ((u32::from(raw) * 100 + u32::from(self.quirks.raw_max) / 2)
            / u32::from(self.quirks.raw_max)) as u8
    }
    fn fan_rpm(percent: u8) -> f64 {
        if percent < 15 {
            0.0
        } else {
            MAX_RPM * (0.2 + 0.8 * f64::from(percent) / 100.0)
        }
    }
    fn slot(&self, ch: usize) -> (usize, u8) {
        let g = self.groups.iter().position(|g| g.contains(&ch)).unwrap();
        let bit = self.groups[g].iter().position(|&c| c == ch).unwrap();
        (g, 1 << bit)
    }
    fn software(&self, ch: usize) -> bool {
        let (g, bit) = self.slot(ch);
        self.mode[g] & bit != 0
    }
    /// Duty the fan actually follows.
    fn effective(&self, ch: usize) -> u8 {
        if !self.software(ch) || self.quirks.ignored_channels.contains(&ch) {
            return FIRMWARE_PERCENT;
        }
        self.decode(self.raw[ch])
            .max(self.quirks.min_effective_percent)
    }
    fn advance(&mut self, now_ms: u64) {
        let dt = now_ms.saturating_sub(self.last_ms) as f64;
        self.last_ms = now_ms;
        for ch in 0..self.raw.len() {
            let target = if self.blocked[ch] {
                0.0
            } else {
                Self::fan_rpm(self.effective(ch))
            };
            let k = if self.quirks.rpm_lag_ms == 0 {
                1.0
            } else {
                1.0 - (-dt / self.quirks.rpm_lag_ms as f64).exp()
            };
            self.rpm[ch] += (target - self.rpm[ch]) * k;
        }
    }
    fn readback(&self, ch: usize) -> Option<u16> {
        self.quirks.readback.then_some(self.raw[ch])
    }
}

/// What the freeRGB worker does through LHM: capture, write, return.
struct Session {
    ch: usize,
    original_bit: bool,
    original_raw: u16,
}

impl Session {
    fn take(chip: &mut SimChip, ch: usize) -> Result<Self, String> {
        if chip.software(ch) {
            return Err("channel already in software mode (another tool)".into());
        }
        let session = Self {
            ch,
            original_bit: false,
            original_raw: chip.raw[ch],
        };
        // Read-modify-write: only this channel's bit changes.
        let (g, bit) = chip.slot(ch);
        chip.mode[g] |= bit;
        Ok(session)
    }
    fn write(&self, chip: &mut SimChip, percent: u8) -> Result<(), String> {
        let raw = SimChip::encode(&chip.quirks, percent);
        chip.raw[self.ch] = raw;
        chip.writes.push((self.ch, percent));
        match chip.readback(self.ch) {
            Some(read) if read != raw => Err("register readback mismatch".into()),
            _ => Ok(()),
        }
    }
    /// Fail-safe return: 100% first, then the captured firmware mode.
    fn fail_safe(self, chip: &mut SimChip) {
        self.write(chip, 100).unwrap();
        self.release(chip);
    }
    fn release(self, chip: &mut SimChip) {
        let (g, bit) = chip.slot(self.ch);
        chip.raw[self.ch] = self.original_raw;
        if self.original_bit {
            chip.mode[g] |= bit;
        } else {
            chip.mode[g] &= !bit;
        }
    }
}

fn firmware_restored(chip: &SimChip, ch: usize) -> bool {
    !chip.software(ch) && chip.raw[ch] == SimChip::encode(&chip.quirks, FIRMWARE_PERCENT)
}

/// Runs the guided channel test on `ch` and returns the calibration output.
fn channel_test(chip: &mut SimChip, ch: usize) -> freergb_control::calibration::CalibrationOutput {
    measure(
        chip,
        ch,
        Calibration::channel_test(ChannelRole::Fan, 85.0, 0).unwrap(),
    )
}

/// Drives any measurement (channel test, calibration, sweep) to its end.
fn measure(
    chip: &mut SimChip,
    ch: usize,
    mut test: Calibration,
) -> freergb_control::calibration::CalibrationOutput {
    let session = Session::take(chip, ch).unwrap();
    let mut applied = None;
    let mut now = 0;
    loop {
        let out = test.output();
        if matches!(
            out.phase,
            CalibrationPhase::Completed
                | CalibrationPhase::Aborted
                | CalibrationPhase::RestorationFailed
        ) {
            session.release(chip);
            return out;
        }
        if applied != Some(out.sequence) {
            session.write(chip, out.duty_percent).unwrap();
            applied = Some(out.sequence);
        }
        now += STEP_MS;
        chip.advance(now);
        test.update(CalibrationSample {
            at_ms: now,
            applied_sequence: applied,
            rpm: Some(chip.rpm[ch] as f32),
            rpm_age_ms: 0,
            temperature_c: Some(40.0),
            temperature_age_ms: 0,
        });
        assert!(now < 300_000, "measurement did not finish");
    }
}

fn fan(test_passed: bool, unlocked: bool) -> ChannelEvidence {
    ChannelEvidence {
        role: Some(ChannelRole::Fan),
        test_passed,
        unlocked,
        pump_confirmed: false,
    }
}

fn controllable() -> Vec<Family> {
    families()
        .into_iter()
        .filter(|f| f.capability != SupportTier::ReadOnly)
        .collect()
}

#[test]
fn fixture_lists_every_required_family() {
    let ids: Vec<String> = families().into_iter().map(|f| f.id).collect();
    for required in [
        "nct67xx_separate",
        "nct6701d",
        "nct668x",
        "ite_it87_ext",
        "ite_it87_legacy",
        "fintek_f718xx",
        "winbond_w836xx",
        "asus_ec",
    ] {
        assert!(ids.iter().any(|i| i == required), "{required}");
    }
}

#[test]
fn guided_channel_test_passes_responsive_channels_and_rejects_quirky_ones() {
    for family in controllable() {
        let mut chip = SimChip::new(&family);
        for ch in 0..family.controls {
            let out = channel_test(&mut chip, ch);
            let ignored = family.quirks.ignored_channels.contains(&ch);
            let label = format!("{} ch{ch}", family.id);
            assert!(firmware_restored(&chip, ch), "{label}: firmware mode");
            if ignored || family.quirks.inverted {
                // No plausible falling response: the test must not pass, so the
                // channel keeps 100% without an unrestricted unlock.
                let passed = out.result.as_ref().is_some_and(|r| r.channel_test_passed());
                assert!(!passed, "{label}: {out:?}");
                let l = limits(family.capability, fan(passed, false)).unwrap();
                assert_eq!(l.floor_percent, 100, "{label}");
            } else {
                assert_eq!(out.phase, CalibrationPhase::Completed, "{label}: {out:?}");
                let result = out.result.unwrap();
                assert!(result.channel_test_passed(), "{label}: {result:?}");
                let l = limits(family.capability, fan(true, false)).unwrap();
                assert_eq!(l.floor_percent, SAFE_FAN_FLOOR, "{label}");
            }
        }
    }
}

#[test]
fn shared_mode_registers_keep_sibling_channels_untouched() {
    for family in controllable() {
        for group in family
            .quirks
            .shared_mode_groups
            .iter()
            .filter(|g| g.len() > 1)
        {
            let mut chip = SimChip::new(&family);
            let (own, sibling) = (group[0], group[1]);
            let session = Session::take(&mut chip, own).unwrap();
            session.write(&mut chip, 80).unwrap();
            assert!(
                !chip.software(sibling),
                "{}: sibling stays firmware",
                family.id
            );
            assert!(firmware_restored(&chip, sibling));
            // A sibling owned by another tool is still refused.
            let (g, _) = chip.slot(sibling);
            let foreign = chip.mode[g];
            let other = Session::take(&mut chip, sibling).unwrap();
            assert!(Session::take(&mut chip, sibling).is_err(), "{}", family.id);
            session.release(&mut chip);
            assert!(
                chip.software(sibling),
                "{}: release keeps sibling bit",
                family.id
            );
            other.release(&mut chip);
            assert_eq!(chip.mode[g] & !foreign, 0);
            assert!(firmware_restored(&chip, own) && firmware_restored(&chip, sibling));
        }
    }
}

#[test]
fn runtime_plausibility_faults_lhm_supported_and_warns_unrestricted() {
    for family in controllable().into_iter().filter(|f| !f.quirks.inverted) {
        let ch = (0..family.controls)
            .find(|c| !family.quirks.ignored_channels.contains(c))
            .unwrap();
        for unlocked in [false, true] {
            let mut chip = SimChip::new(&family);
            let result = channel_test(&mut chip, ch).result.unwrap();
            let l = limits(family.capability, fan(true, unlocked)).unwrap();
            let mut plausibility = RpmPlausibility::new(&result.tested).unwrap();
            let session = Session::take(&mut chip, ch).unwrap();
            session.write(&mut chip, 60).unwrap();
            let mut now = 0;
            let mut deviated = None;
            for step in 0..120u64 {
                now = step * STEP_MS;
                if step == 60 {
                    chip.blocked[ch] = true;
                }
                chip.advance(now);
                if plausibility.observe(60, chip.rpm[ch], now) {
                    deviated = Some(now);
                    break;
                }
            }
            let label = format!("{} unlocked={unlocked}", family.id);
            let at = deviated.unwrap_or_else(|| panic!("{label}: no deviation at {now}"));
            assert!((20_000..=21_500).contains(&at), "{label}: {at}");
            if l.plausibility_fatal {
                session.fail_safe(&mut chip);
                assert!(firmware_restored(&chip, ch), "{label}");
                assert_eq!(chip.writes.last(), Some(&(ch, 100)));
            } else {
                // Verified/unrestricted channels only warn; control continues.
                assert!(chip.software(ch), "{label}");
                session.release(&mut chip);
            }
            assert_eq!(
                l.plausibility_fatal,
                family.capability == SupportTier::LhmSupported && !unlocked,
                "{label}"
            );
        }
    }
}

#[test]
fn stale_or_critical_telemetry_returns_every_family_to_firmware() {
    for family in controllable() {
        for fault in [TelemetryFault::Stale, TelemetryFault::Critical] {
            let mut chip = SimChip::new(&family);
            let ch = 0;
            let mut guard = TelemetryGuard::new("temp".into(), "rpm".into(), 85.0).unwrap();
            let session = Session::take(&mut chip, ch).unwrap();
            session.write(&mut chip, 50).unwrap();
            let mut got = None;
            for step in 1..40u64 {
                let now = step * STEP_MS;
                chip.advance(now);
                let temp = if fault == TelemetryFault::Critical && step > 10 {
                    90.0
                } else {
                    45.0
                };
                let fresh = fault != TelemetryFault::Stale || step <= 10;
                let mut result = Ok(());
                if fresh {
                    result = guard
                        .observe(
                            Reading {
                                source: "temp",
                                acquired_ms: now,
                                value: temp,
                            },
                            now,
                        )
                        .and_then(|_| {
                            guard.observe(
                                Reading {
                                    source: "rpm",
                                    acquired_ms: now,
                                    value: chip.rpm[ch] as f32,
                                },
                                now,
                            )
                        });
                }
                if let Err(f) = result.and_then(|_| guard.check(now)) {
                    if step > 1 {
                        got = Some(f);
                        break;
                    }
                }
            }
            assert_eq!(got, Some(fault), "{}", family.id);
            session.fail_safe(&mut chip);
            let n = chip.writes.len();
            assert_eq!(
                chip.writes[n - 1],
                (ch, 100),
                "{}: 100% before release",
                family.id
            );
            assert!(firmware_restored(&chip, ch), "{}", family.id);
        }
    }
}

#[test]
fn unrestricted_unlock_allows_stop_but_pumps_and_untested_channels_stay_full() {
    for family in controllable() {
        let l = limits(family.capability, fan(false, true)).unwrap();
        assert_eq!(l.tier, SupportTier::Unrestricted, "{}", family.id);
        assert!(l.allow_stop && l.floor_percent == 0);
        let pump = ChannelEvidence {
            role: Some(ChannelRole::Pump),
            test_passed: true,
            unlocked: true,
            pump_confirmed: false,
        };
        assert_eq!(limits(family.capability, pump).unwrap().floor_percent, 100);
        let untested = limits(family.capability, fan(false, false)).unwrap();
        let expected = if family.capability == SupportTier::Verified {
            SAFE_FAN_FLOOR
        } else {
            100
        };
        assert_eq!(untested.floor_percent, expected, "{}", family.id);

        // 0% on an unlocked channel: a stop is a warning, never a fault.
        let mut chip = SimChip::new(&family);
        let ch = 0;
        let mut guard = TelemetryGuard::new("temp".into(), "rpm".into(), 85.0)
            .unwrap()
            .allow_fan_stop();
        let session = Session::take(&mut chip, ch).unwrap();
        session.write(&mut chip, l.floor_percent).unwrap();
        for step in 1..60u64 {
            let now = step * STEP_MS;
            chip.advance(now);
            guard
                .observe(
                    Reading {
                        source: "temp",
                        acquired_ms: now,
                        value: 40.0,
                    },
                    now,
                )
                .unwrap();
            guard
                .observe(
                    Reading {
                        source: "rpm",
                        acquired_ms: now,
                        value: chip.rpm[ch] as f32,
                    },
                    now,
                )
                .unwrap();
            guard.check(now).unwrap();
        }
        let stopped = chip.effective(ch) < 15;
        assert_eq!(guard.warning().is_some(), stopped, "{}", family.id);
        if family.quirks.min_effective_percent >= 15 {
            assert!(
                !stopped,
                "{}: limited duty range clamps the stop",
                family.id
            );
        }
        session.release(&mut chip);
        assert!(firmware_restored(&chip, ch));
    }
}

#[test]
fn read_only_families_expose_no_controls() {
    for family in families()
        .into_iter()
        .filter(|f| f.capability == SupportTier::ReadOnly)
    {
        assert_eq!(family.controls, 0, "{}", family.id);
        assert!(limits(family.capability, fan(true, true)).is_err());
    }
}

#[test]
fn verified_families_detect_a_write_that_does_not_stick() {
    for family in controllable().into_iter().filter(|f| f.quirks.readback) {
        let mut chip = SimChip::new(&family);
        let session = Session::take(&mut chip, 0).unwrap();
        session.write(&mut chip, 70).unwrap();
        // Firmware overwrites the duty register: readback must disagree.
        chip.raw[0] = SimChip::encode(&chip.quirks, FIRMWARE_PERCENT);
        let raw = SimChip::encode(&chip.quirks, 70);
        assert_ne!(chip.readback(0), Some(raw), "{}", family.id);
        session.release(&mut chip);
    }
}

fn responsive(family: &Family, ch: usize) -> bool {
    !family.quirks.ignored_channels.contains(&ch) && !family.quirks.inverted
}

#[test]
fn fine_stall_sweep_finds_thresholds_on_every_family() {
    use freergb_control::calibration::SweepOptions;
    let options = SweepOptions {
        step_percent: 5,
        lowest_percent: 0,
        probe_stall: true,
    };
    for family in controllable() {
        let mut chip = SimChip::new(&family);
        for ch in (0..family.controls).filter(|&ch| responsive(&family, ch)) {
            let label = format!("{} ch{ch}", family.id);
            let sweep = Calibration::sweep(ChannelRole::Fan, 85.0, 0, options).unwrap();
            let out = measure(&mut chip, ch, sweep);
            assert!(firmware_restored(&chip, ch), "{label}: firmware mode");
            assert_eq!(out.phase, CalibrationPhase::Completed, "{label}: {out:?}");
            let result = out.result.unwrap();
            assert!(result.tested.iter().all(|t| t.rpm >= 100.0), "{label}");
            if family.quirks.min_effective_percent >= 15 {
                // The chip never lets the fan stop: no stall threshold.
                assert_eq!(result.stall_percent, None, "{label}");
            } else {
                // The simulated fan stops below 15%.
                assert_eq!(result.stall_percent, Some(10), "{label}: {result:?}");
                assert_eq!(result.start_percent, Some(15), "{label}");
                assert_eq!(result.suggested_floor_percent, Some(15), "{label}");
                assert!(result.monotonic_response, "{label}");
                assert_eq!(result.tested.len(), 18, "{label}: {result:?}"); // 100..15 in 5% steps
            }
        }
    }
}

#[test]
fn shaped_closed_loop_rpm_target_converges_on_every_family() {
    use freergb_control::calibration::TestedDuty;
    use freergb_control::rpm::{PiGains, RpmMap, RpmPi};
    use freergb_control::shaping::{
        AvoidRange, AvoidUnit, ChannelLimits, OutputShaper, OutputShaping,
    };
    for family in controllable() {
        let mut chip = SimChip::new(&family);
        for ch in (0..family.controls).filter(|&ch| responsive(&family, ch)) {
            let label = format!("{} ch{ch}", family.id);
            // A map measured when the fan was 10% faster (dust, wear, voltage).
            let tested: Vec<TestedDuty> = [100u8, 80, 60, 40]
                .into_iter()
                .map(|d| TestedDuty {
                    duty_percent: d,
                    rpm: SimChip::fan_rpm(d.max(family.quirks.min_effective_percent)) as f32 * 1.1,
                })
                .collect();
            let map = RpmMap::from_tested(&tested).unwrap();
            let shaping = OutputShaping {
                avoid: vec![AvoidRange {
                    from: 85.0,
                    to: 95.0,
                    unit: AvoidUnit::Percent,
                }],
                hysteresis_up_percent: 0.5,
                hysteresis_down_percent: 0.5,
                ..OutputShaping::default()
            };
            let limits = ChannelLimits {
                floor_percent: f32::from(SAFE_FAN_FLOOR),
                allow_stop: false,
            };
            let session = Session::take(&mut chip, ch).unwrap();
            let (mut pi, mut shaper) = (RpmPi::default(), OutputShaper::default());
            let target = 1200.0;
            let mut now = 0;
            for _ in 0..400 {
                now += STEP_MS;
                chip.advance(now);
                let raw = pi
                    .update(
                        &map,
                        PiGains::default(),
                        target,
                        Some(chip.rpm[ch] as f32),
                        0,
                        now,
                    )
                    .duty_percent;
                let duty = shaper.apply(&shaping, raw, false, limits, now).ceil() as u8;
                assert!(duty >= SAFE_FAN_FLOOR, "{label}");
                assert!(
                    !(86..95).contains(&duty),
                    "{label}: duty {duty} in avoided range"
                );
                session.write(&mut chip, duty).unwrap();
            }
            let rpm = chip.rpm[ch] as f32;
            assert!((rpm - target).abs() < target * 0.03, "{label}: {rpm} RPM");
            // Lost telemetry: open loop again (the map's duty, not the PI duty).
            let open = pi.update(&map, PiGains::default(), target, None, 0, now + STEP_MS);
            assert!(!open.closed_loop, "{label}");
            assert_eq!(open.duty_percent, map.duty_for(target).unwrap(), "{label}");
            session.fail_safe(&mut chip);
            assert!(firmware_restored(&chip, ch), "{label}");
        }
    }
}
