# Fan-control defaults: code review

Reviewed 2026-11-08, `task/legal-beta`; source review only, no hardware activity.
References below use file paths and symbols so they survive line movement.
These observations do not establish physical RPM or recovery on a given device.

| Requirement | Source evidence | Result and limits |
| --- | --- | --- |
| Default OFF / explicit opt-in | `crates/freergb-daemon/src/onboarding.rs`: `OnboardingState::default`, `require_safety`, `acknowledge`; `api.rs` calls `require_safety`; `engine.rs`: `start_cooling`, `cooling_policy`, `resume_cooling`; `main.rs`: hwmon `start(true, onboarding.safety_acknowledged())` | Fresh state has no acknowledgement and fan writes are gated. Acceptance requires the current text version and accepted=true. This is an acknowledgement gate, not a universal persistent master OFF switch. Previously configured jobs/profiles may resume after restart. |
| Resume / revoke | `engine.rs`: `resume_cooling`; `hwmon.rs`: `start`; `api.rs`: onboarding revoke and associated regression tests | Startup can restore opted-in assignments. Hwmon checks the current acknowledgement and board identity before applying saved assignments. Revoke drains sessions; reopening the wizard alone retains acknowledgement (`onboarding.rs::reset`). |
| PWM floor and pump policy | `crates/freergb-control/src/fan_support.rs`: `limits`; `crates/freergb-daemon/src/fan_support.rs`: `FanSupportConfig::default`, identity-bound unlock records | Normal eligible fans have 40% duty floor; pumps, unknown roles and channels without required evidence stay at 100%. Unrestricted defaults false; unlocked channels can have 0% floor; pump reduction requires separate confirmation. This policy must not be described as a guaranteed minimum RPM. |
| RPM limits | `crates/freergb-control/src/rpm.rs`: `RpmMap::from_tested`, `duty_for`, `RpmPi::update`; `calibration.rs`: validation; `shaping.rs`: `validate_for` | At least two monotonic calibration points required; readings below 100 RPM rejected as implausible. Requested targets below the slowest measured point are refused; stale/invalid RPM falls back to the mapped open-loop duty. No universal physical stall detector or guaranteed actual minimum RPM follows from this. Shaping rejects fan-stop configuration on pumps. |
| Firmware / BIOS fallback | `crates/freergb-daemon/src/hwmon.rs`: `restore`, recovery journal; `gpu_cooling.rs`: journal/release; `providers/mainboard/NativeControlSession.cs`: native session fault/return paths | Linux attempts captured firmware-mode restore; GPU attempts driver auto/previous-mode return. Durable journals retain unresolved returns for restart recovery. Windows lower tiers attempt firmware return; verified-tier emergencies can request 100% and retain reservation. No unconditional BIOS fallback on process/OS/driver/power failure; a dead process cannot issue a command. |
| Warning before unrestricted unlock | `apps/desktop/src/features/cooling/UnrestrictedSection.tsx`; locale keys `cooling.unrestrictedSection.*` in `apps/desktop/src/lib/i18n/locales/en.json` and `de.json`; `crates/freergb-daemon/src/fan_support.rs`: `unlock_channel`, `confirm_pump` | Warning banner, default-off global switch, typed channel name, risk checkbox and separate PUMP confirmation. Server validates switch/name/identity/pump confirmation; UI checkbox is a UI condition, not an independently provable reading/understanding check. |

## Follow-up / evidence gaps

See additions to `docs/promts/fancontrol/07-hardware-acceptance.md` for packages
and owner-supervised hardware work. In particular: default-off is established
for fresh state through gates, not a guarantee that an existing installation
never resumes; percentage floors do not prove a physical RPM threshold;
return requests do not prove firmware has regained control. Backend-specific
policies require separate evidence. No changes were made to controller logic,
modules, existing acknowledgement text or its version in this package.
