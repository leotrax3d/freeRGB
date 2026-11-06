# Cooling acceptance record — 2026-10-22

This record separates the user's existing hardware results from verification of
the current source. No broad board compatibility or FanControl parity is implied.

## Existing hardware evidence

ASUS PRIME X870-P WIFI, Nuvoton NCT6701D, normal fan on CHA_FAN2:
`/lpc/nct6701d/0/control/2`, RPM `/lpc/nct6701d/0/fan/2`.
The other motherboard tachometer belongs to the Corsair iCUE LINK AIO and must
not be reduced as a normal fan. SuperIO temperature labels are not verified CPU
or coolant identities.

The protected Rust → service → native-worker path executed an 80% curve and
returned to firmware with checked registers and journal clearance. Calibration
executed 100/80/60/40%; its 50% recommendation intentionally has
`hardware_verified=false`. These observations were supplied in the task handoff;
they were not repeated during this source integration.

An active 80% session was drained before a same-version protected service payload
replacement. This proves neither release-to-release upgrade nor rollback nor
uninstallation. The bench-specific script is
`scripts/test-mainboard-active-upgrade.ps1`; it requires explicit probe consent.

## Current source integration

The additive session API and Classic dialog call the daemon-owned Rust runtime.
Normal setup does not grant control authorization. Each start requires explicit
fan/RPM confirmation and Windows UAC. Native telemetry, register validation,
watchdog and recovery journal remain authoritative. `available=true` only means
the daemon worker was enabled; it is not a hardware or installed-service verdict.

The separate `feature/tauri-gui` worktree was inspected without changing its files.
Its transport invokes the same local daemon API through a serialized Rust queue;
shutdown methods are excluded. Its cooling view currently offers curves and
source preview, without a live-session dialog. No Tauri/Node dependencies or
`apps/desktop` files were added to main.

## Release and hardware gates

- A live session through the new public API/Classic dialog still needs interactive
  UAC acceptance and verified firmware return on the exact confirmed fan.
- Sustained operation, temperature/RPM fault injection, reboot, native/driver hang,
  service/OS crash and power loss require controlled hardware acceptance.
- Real version-to-version upgrade, rollback, rejected/timeout maintenance, and
  actual uninstall must be accepted on a disposable release test installation.
- Wider chips/boards and motherboard RGB/ARGB require separate hardware evidence.
- The 55% overall progress value remains a rough engineering estimate, not a
  measured result or compatibility percentage.

Setup now prepares protected maintenance before replacing application files.
The service closes admission, drains the worker, verifies return and clears the
journal; the installer then checks the journal under the native maintenance mutex.
Protected payload replacement/removal retains that mutex throughout its transaction.
Refusal or missing acknowledgement stops maintenance without deleting recovery
evidence. Cancellation after preparation leaves admission closed until a service
restart; never restart around an unresolved recovery journal. Old services without
the handshake can pass only if their existing native journal/mutex gate is clear.

## Verification of this integration

Local checks on 2026-10-22:

- `cargo check -p freergb-daemon -p freergb-gui --all-targets`: passed;
  the initial unused import warning was fixed.
- `cargo clippy -p freergb-daemon -p freergb-gui --all-targets --locked -- -D warnings`: passed.
- Daemon library: 51 passed, 1 pre-existing provider fixture ignored.
- Classic GUI: 14 passed; launcher: 2 passed. Debug daemon, GUI, launcher and CLI built.
- Native control safety/session/journal tests and broker boundary tests: passed
  with synthetic ports/test storage; no hardware or service installation.
- Deployment transactions: passed, including synthetic failed-start rollback;
  no real SCM/active hardware rollback acceptance.
- Rust to C# interoperability: all 6 tests passed over real current-user pipes
  with synthetic native measurements, including calibration and sampler loss.
- `scripts/test-cooling-session-api.ps1`: passed against a real isolated daemon
  named pipe in `--no-hardware --no-state` mode. Disabled start/stop, missing and
  unknown fields and owned shutdown were checked; no UAC or hardware writes.
- Tauri branch: frontend build passed, 56 Vitest tests passed, 5 Rust tests passed.
  Tracked files were unchanged by inspection/build/testing.
- Inno Setup 6.7.3 installed through winget (verified package hash). Installer
  compiled using current debug binaries into `target/installer-validation/`;
  this is a validation artifact, not an optimized/signed release. Installer
  command/matching model tests passed. Setup itself was not installed or uninstalled.

The fresh isolated API smoke check is included in Windows CI after the release
build. CI status for the eventual commit must be checked separately; local Windows
results do not establish Linux success or disposable-runner SCM acceptance.

## Concurrent documentation reconciliation

The incoming main branch introduced an automated delivery checklist in STATUS.md.
Its 51.4% (19/37 equally weighted milestones) measures checklist completion, not
engineering effort. The earlier 55% estimate retained in the SVG is a different,
rough historical estimate. README keeps the new generated checklist; no combined
acceptance milestone was checked merely because the API/Classic implementation
now builds. Incoming branding, website and README structure were preserved.

## Owner acceptance session 2026-11-05 (prompt 07)

Board ASUS PRIME X870-P WIFI, Nuvoton NCT6701D; installed with local installers
built from `main` and `cooling/fix-reader-hang`. Fans stayed under firmware control
throughout block A; no hardware writes.

| Step | Build | Result | Observation |
|---|---|---|---|
| A0 upgrade over beta.4 | `c8b79fa` | pass | Setup replaced files, verified LHM, reinstalled the protected service; PawnIO and .NET already present. |
| A1 discovery | `c8b79fa` | **fail** | Protected reader never delivered a first snapshot ("Motherboard reader is starting" for > 20 min). Cause: LHM storage enumeration (`IsStorageEnabled`, added with the sensor catalog) blocks `Computer.Open` in the LocalSystem worker; the 10 s first-reply timeout never fired on the synchronous redirected stdout. Fixed in `44781c2` (storage off, kill watchdog). |
| A1 discovery | `44781c2` | **fail** | Broker now answers `ready` when the helper is run directly, but the daemon stayed `permission_denied`: it started ~3 s before the broker and its provider thread never retried a non-ready reply. Fixed in `79c2972` (retry every 5 s). |
| A1 discovery | `79c2972` | **fail, then pass** | After setup the broker's single pipe instance stayed busy (ERROR_PIPE_BUSY 231) until `Restart-Service freeRGBMainboard`; afterwards `ready` within seconds. Root cause of the stuck instance is open. |
| A1 inventory | `79c2972` | pass | 7 NCT6701D controls (tier `verified`, coverage `hardware-verified`), CHA_FAN2 = `control/2` (30 %, 697 rpm), second tachometer `fan/1` 1790 rpm (AIO pump). LHM also reports `/gpu-amd/5` fan/control. |
| A1 pump safety | `79c2972` | pass | Policy: channels with role `pump` or no role have a 100 % floor (only an explicit unrestricted unlock plus a typed `PUMP` confirmation lifts it; unrestricted is off). Owner confirmed `control/1`/`fan/1` as the AIO pump; bindings saved: `control/1` pump "AIO_PUMP (Fan #2)", `control/2` fan "CHA_FAN2". Not automatically detected: guided setup must ask for it. |
| B1 fine sweep | `79c2972` | **fail** | `mode:calibration` + `sweep:{step_percent:5,lowest_percent:40}` on CHA_FAN2 (Tctl source, critical 80 °C) completed after ~10 s with `minimum_applied_percent: 95`, `calibration: null`, no fault/warning; nothing stored (`cooling-fan-support.json` absent, `rpm_range` null). Return to firmware verified by register readback; pump untouched (1749 rpm). Jobs 1–4 (two curves, two calibrations, all 100 %) existed before the first request of this session; origin unknown (desktop GUI open). |
| R5 fine sweep | `cfe8400` | **fail** | Aborted `unstable_rpm` at 100%: fixed 2 s settle while the fan still accelerates (830 -> ~1690 rpm); returned by register readback. Fix: adaptive bounded RPM settling (fix commit pending coordinator commit), covered by ramp, oscillation, timeout and 13-step NCT6701D simulator tests. |
| B2–G | — | not run | Stopped: owner away; every session needs an interactive UAC authorization by design. |

### Fixes for the 2026-11-05 findings (prompt 08, simulated; owner re-test in prompt 07 block R)

| Finding | Fix commit | Test |
|---|---|---|
| Reader stuck at "starting" | `44781c2`, `8503a70` | `BrokerTests.SilentWorkerTests` (silent worker -> "reader failed") |
| Provider never retried | `79c2972`, `8503a70` | `helper_transport...` scenario `denied_then_ready` |
| Broker pipe busy after setup | `8503a70` | `BrokerTests` stuck-client case (5 s deadline, next client served) |
| B1 sweep stored nothing | `fc31782` | `fine_sweep_runs_all_thirteen_steps...`, `plain_calibration_completes...`; abort reason now in `report.fault` |
| Unexplained jobs | `984c041` | `every_job_reports_who_started_it` (`origin`) |
| Pump not detected | `a3f677b` | `setupModel.test.ts` role gate |
| Second installer | this PR | `SetupMutex` in `freergb.iss` (owner step R1) |

B1 root cause in simulation: a fan that saturates near 100 % failed the
1 %-per-step monotonic rule, so the result was dropped. The owner run also
ended after the 95 % step with `calibration: null`; the abort reason was not
reported then and is now. If R5 still ends early, its `report.fault` names why.

### Owner evening session 2026-11-05 (build 984c041 and an experimental variant without LHM GPU)

| Step | Result | Notes |
|---|---|---|
| R2 broker after setup | **pass** | Ready right after setup, no `Restart-Service`. |
| R4 job origin | **pass** | Jobs carry `origin: "api"`. |
| R5 fine sweep | **fail** | Ended "blocked: Cooling supervision stopped: Stopped, Some(Stale); preserve recovery intent"; channel returned cleanly, recovery null. Cause: reader flapping (item 8). |
| Reader stability | **fail** | Over 2 min: 14/24 snapshots ready with iCUE running, 14/24 with LHM GPU disabled, 20/24 with iCUE closed (CorsairCpuIdService stopped). "Isolated motherboard reader failed (TimeoutException)" / "sample expired" / daemon "helper timed out". Ring0/PawnIO contention contributes, not the only cause. |
| D6 maintenance drain | **fail** | PrepareToInstall "Mainboard-Rückgabe nicht bestätigt"; `mainboard-maintenance-result.json` failed ("Native return failed; maintenance refused."), last error stage admission/IOException; background control/2 at 47 %. Install succeeded after `cooling.background.apply {channels:[]}`. |
| Bindings | **fail** | A third binding in channel-id spelling (`["/lpc/nct6701d/0","…/control/2"]`, header "Fan #3") duplicated the plain-id CHA_FAN2 binding. |

Fixes (simulated; re-test in prompt 07 block R): item 8 (reader update
budget, deadlines), item 9 (drain stops background curves first), item 10
(binding canonicalization) — see the branch `cooling/fix-reader-hang` log.
The experimental installer without LHM GPU on the owner PC is not committed.
