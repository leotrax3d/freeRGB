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

## Owner session 2026-11-06: R5 calibration settling

Owner hardware: ASUS PRIME X870-P WIFI, NCT6701D, CHA_FAN2
`/lpc/nct6701d/0/control/2`. A calibration sweep with
`step_percent:5,lowest_percent:40` faulted after about 10 seconds:
`Calibration aborted: inconsistent_response`, minimum applied duty 95%,
initial RPM 1002, last RPM 1709. Earlier channel-test readings were 1685 RPM
at 100%, 1599 at 70%, and 1222 at 50%: a flat top and slow settling.

The fixed two-second wait sampled the initial 100% stage while the fan was
still accelerating from firmware duty. The subsequent 95% stage then exceeded
that underestimated baseline by more than 10%, triggering the consistency guard.

Fix commit: `298225eb9e5964aabbd2579a7f7c582705b4fbfe`
(Wait for stable RPM during fan calibration). Calibration now waits for five
distinct readings at least 250 ms apart, within a 3% band and without monotonic
drift above 0.5%, before accepting their mean. The first stage allows 15 seconds
after acknowledgment, subsequent stages 10 seconds; failure to settle restores
full speed with `unstable_or_missing_samples`. The overall budget covers these
stage deadlines and restart probes. Pump refusal, duty floors, temperature and
telemetry guards, acknowledgment deadlines and restoration remain enforced.

Regression coverage is simulated only: six- and ten-second 1000-to-1700 RPM
ramps followed by flat 1700 RPM at 95/90% complete all 13 steps; a genuinely
increasing settled response still restores and aborts. Narrow monotonic drift
times out at both stage bounds. The NCT6701D register simulator covers the
six-second ramp, floor, untouched pump channel and firmware return.

Owner re-test remains open: with the owner present, repeat R5 on CHA_FAN2
using 5% steps down to 40%; confirm the initial stage waits for steady RPM,
all 13 duties are recorded, a result is stored and the channel returns to
firmware with a clear recovery journal. No hardware, UAC, installer or installed
application files were accessed for this fix. `hardware_verified` remains false.

Local validation on 2026-11-06:

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test -p freergb-control -p freergb-daemon --locked`: passed, including
  doctests (225 tests passed, 7 existing fixture-dependent tests ignored).
  An initial run overlapping Clippy failed at Rustdoc dependency resolution;
  the full command passed when rerun without another Cargo build in parallel.

## Owner session 2026-11-06: R5b loaded linear-session freshness

On the same ASUS PRIME X870-P WIFI / NCT6701D CHA_FAN2 channel, the owner ran
`mode:linear` during a heavy `cargo build` (Tctl 73 C). About ten seconds into
execution the session became blocked with `Cooling supervision stopped: Stopped,
Some(Stale); preserve recovery intent`. The inventory reader had been ready for
24/24 idle observations. No new hardware run was performed during this fix.

The exact message identifies the daemon's `SupervisedControl` / `TelemetryGuard`
1500 ms acquisition-age check, which stops the lease with `TelemetryFault::Stale`.
It does not identify a broker timeout or a native-worker emergency receipt.
`CoolingHost::start_native` takes live data from authenticated native-worker
receipts; it does not consume the broker's inventory snapshots. The broker sleeps
one second after each snapshot and expires cached data after five seconds.
The dedicated native sampler instead sleeps 100 ms, then refreshes checked SuperIO
RPM and the selected temperature hardware (Tctl uses its CPU hardware). Both its
independent C# guard and the Rust receipt decoder also enforced 1500 ms.

Acquisition is conservatively timestamped before the hardware refresh. With two
consecutive three-second reads, the last published measurement reaches about
6100 ms old before the next read finishes. The source trace establishes this
budget mismatch; the owner's error contains no per-read durations, so the exact
hardware delay remains to be measured during re-test.

Fix: **Align native telemetry freshness with slow reader cadence** (this commit).
The native C# guard, Rust receipt decoder and native-session Rust supervision now
share a seven-second acquisition-age ceiling. A 2-3 s read therefore fits without
inventing newer timestamps. Generic Rust samplers keep their 1500 ms default;
broker expiry and calibration measurement-age rules are unchanged.

Fail-safe remains bounded: linear/curve temperature age >3000 ms still requests
100% and bypasses shaping; observed critical heat still acts immediately. At
>7000 ms acquisition age, stale is latched and the session ends. Explicit native
close verifies firmware return; an independent native fault retains the existing
full-speed/recovery policy for verified channels and firmware callback for lower
tiers. A fresh command or cached receipt cannot renew sensor freshness. The
one-second command lease, receipt deadlines, pump/floor checks and durable recovery
requirements are unchanged.

Regression: a real Rust duplex session with native telemetry receipts models a
73 C linear curve, 2/3/3-second acquisitions, the 100 ms sampling pause and cached
sample identities. It survives every slow update, requests full speed during
delays, then returns with register/journal evidence when acquisition freezes,
despite continuing command heartbeats. Rust policy tests cover independent source
expiry and immediate critical heat; C# synthetic session tests cover slow reads,
the exact 7000/7001 ms boundary, full-speed stale faults and non-revival.

Owner re-test remains open: repeat the loaded linear session on confirmed CHA_FAN2
with the owner present, track native sample age and session status for at least
two minutes, confirm 100% during delayed telemetry, stop explicitly and verify
firmware return plus a clear journal. This is simulated coverage, not hardware
acceptance. No UAC, installer or installed application files were accessed.

Local validation for R5b:

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test -p freergb-control -p freergb-daemon --locked`: passed, including
  doctests (227 passed, 7 existing fixture-dependent tests ignored). An initial
  Rustdoc stale-dependency error resolved after rebuilding doctest artifacts;
  the complete command then passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-mainboard-control.ps1`:
  passed; synthetic native callbacks/ports and test-directory journal storage,
  without opening hardware or installing a service/driver.


## Owner session 2026-11-06: R5c native acknowledgement under CPU load

Owner evidence: ASUS PRIME X870-P WIFI / NCT6701D / CHA_FAN2, four concurrent
Cargo builds, Tctl about 70 C. `mode:calibration` immediately blocked with
`Native acknowledgement deadline exceeded; preserve recovery intent`.
The broker recorded `stage:session`, `kind:EndOfStreamException`; recovery was
null after the channel returned cleanly. Idle sessions started successfully.

The exact daemon message comes from `cooling_runtime`'s **150 ms per-command**
receipt deadline, after `CoolingHost::start_native` has already acquired a client.
It is not the LHM startup timeout. CPU scheduling/IPC latency can exhaust this
small allowance; cancellation drops the relay and explains the broker EOF.
Per-command timings were not recorded, so this is the source-based explanation,
not a measured attribution of the owner's delay to a particular native call.

The startup path also had mismatched nested limits: Rust host 12 s, relay-ready
11 s, broker worker-ready 10 s and initial worker receipt 2 s. `Mainboard.Main`
loads/opens LHM and finds the control before `NativeControlWorker.Run`; that
worker validates its binding and takes its first checked sample before `ready`.
Authenticated acquisition requests 100%, then issues the first ticket and starts
the one-second lease. None of that initialization belongs in a steady-state
command allowance. The first *set command* already uses an active ticket/lease;
it cannot safely take 2-4 s, even if it only requests 100%.

Fix (this commit): bound whole acquisition at 30 s, relay startup at 25 s,
worker initialization at 20 s and initial receipt at 5 s. Both Rust and C# peers
allow the initial receipt budget only once; command reads retain their 2 s
transport bound. Runtime acknowledgements allow 750 ms for scheduling and IPC,
below the independent 1000 ms lease. Calibration acknowledgements remain bounded
at 1000 ms. Tickets remain 250 ms (client sends within 200 ms); this change does
not make a ticket delayed in transit valid. Worker process priority is AboveNormal
before LHM initialization; telemetry and lease-watchdog threads also use
AboveNormal, never realtime. No reconnect, replay or second acquisition is added.

Startup timeout/cancellation closes the original transport and leaves daemon
admission blocked with unresolved recovery intent. EOF alone never proves release.
Once acquired, the native watchdog, critical/stale checks, pump/floor rules and
100%-then-firmware return policy remain in force. Driver hangs still require the
existing external recovery path; priority is not independent fault supervision.

Regression coverage is synthetic only: Rust duplex/host and C# named-pipe fixtures
cover 2/4-second initialization/first-receipt delays and explicit close; a silent
worker times out, closes the same transport and leaves Rust maintenance blocked
with one unresolved token and no return report. Existing anti-replay, slow-write,
lease-expiry, telemetry, disconnect and recovery fixtures continue to apply.

Owner re-test R5c remains open (prompt 07). No hardware, UAC, installer, service
changes or installed application files were accessed. Hardware acceptance and
`hardware_verified` remain unchanged.

Local validation for R5c:

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: failed on
  two pre-existing `modules/drgb` lints: `manual_is_multiple_of`
  (`src/report.rs:167`) and `unnecessary_map_or` (`src/lib.rs:66`). Both expressions
  are unchanged from the parent commit; no unrelated module changes were made.
- `cargo clippy -p freergb-control -p freergb-daemon --all-targets --locked -- -D warnings`:
  passed.
- `cargo test -p freergb-control -p freergb-daemon --locked`: passed (246 tests,
  7 existing fixture-dependent tests ignored, doctests passed). The initial run
  lacked the sibling effect-worker binary required by marketplace tests; after
  `cargo build -p freergb-effect-host --bin freergb-effect-worker --locked`, the
  complete requested command passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-mainboard-control.ps1`:
  passed, including 2/4-second startup and silent-worker fixtures.
- `scripts/build-mainboard-helper.ps1 -OutputDirectory <build-dir>/ack/mainboard`:
  production helper/service compilation and hardware-free serialization self-test
  passed. Nothing was installed.

## Owner session 2026-11-06: R5 control-worker startup and reader contention

| Step | Owner build | Result | Evidence / fix |
| --- | --- | --- | --- |
| R5 fine sweep, two starts | `d61387a` | **fail** | CHA_FAN2 (`/lpc/nct6701d/0/control/2`, RPM `fan/2`, CPU `/amdcpu/0/temperature/2`): authorizing -> executing -> blocked within about five seconds, `Stopped, Some(Stale); preserve recovery intent`. Broker error: `stage:session, kind:EndOfStreamException`. Recovery cleared, firmware mode and reader ready again within three seconds. Fix commit: pending local validation. |

On the ASUS PRIME X870-P WIFI / NCT6701D, without a control session the
inventory reader stayed ready for 24/24 and 8/8 observations over minutes with
iCUE running. The earlier `984c041` session also coincided with reader flapping.
These are owner observations; this change has no new hardware acceptance.

The source confirms that each protected control worker opened motherboard,
CPU and GPU monitoring while the reader had its own LHM Computer. Its 100 ms
sampler refreshed the selected CPU/GPU hardware again. That duplicate work is
a contention path; the logs do not prove the exact driver wait responsible for
the owner's failure. Rust session supervision consumes native receipts rather
than inventory snapshots, so reader flapping alone is not a direct Stale input.

Control workers now open only the motherboard group; CPU, GPU, storage and USB
monitoring are not enabled there. The authenticated SYSTEM broker's reader cache
is the sole CPU/GPU temperature source. SuperIO RPM and on-chip temperature reads
retain their checked ISA access, native control callbacks and recovery evidence.
Reader samples carry a boot-local monotonic acquisition identity through cache
reuse. Cache age uses that acquisition directly instead of counting a slow
snapshot's latency twice; old readers retain their existing age-based contract.

Private native receipts carry optional independent temperature/RPM identities
and ages. Fresh RPM, commands and cache reads cannot renew a frozen temperature.
For hottest-of sources, the published combined temperature stays paired with its
oldest acquisition until that acquisition advances; the native critical-heat
guard still checks each current hottest value immediately. Older receipt shapes
remain supported. Missing, duplicate, invalid, future or expired shared readings
fail closed. The inventory cache retains its five-second expiry and native
supervision retains the seven-second acquisition-age ceiling, independent stall
detection, command lease and tier-specific recovery policy.

Native startup remains bounded at 12 seconds, with no Rust telemetry supervision
until the factory returns its first authenticated sampled receipt. An initial
shared temperature can predate the host: the runtime clock now includes that
acquisition without resetting its age. Regression tests delay startup by eight
seconds, then freeze temperature while RPM and command acknowledgments keep
arriving: startup succeeds, real staleness stops control and an explicit close
returns firmware with journal evidence. Separate tests cover cached acquisition
identity, mixed-source heat, independent stall detection and cache age accounting.

Owner re-test remains open: repeat R5 on confirmed CHA_FAN2 with iCUE running,
observe reader readiness throughout acquisition and the sweep, then verify all
calibration steps, stored results, firmware return and a clear recovery journal.
No UAC, elevation, installer, service changes, installed application files or
hardware access were used in this source work. `hardware_verified` remains false.
