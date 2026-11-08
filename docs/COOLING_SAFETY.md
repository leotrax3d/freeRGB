# Cooling calibration and lease policy

> Engineering documentation, not legal advice and not an approval of any text.
> The in-app safety notice (text version 2) and this page are drafts for a
> lawyer's review. Nothing here says that fan control is safe on any hardware.

## Safety claims by backend

Fan and pump control can overheat and damage hardware. The notice shown before
first use replaced version 1's absolute statement that every channel returns to
firmware on stop, crash, lost sensor data or watchdog expiry. That statement
was not true for every path. Behaviour verified against the merged cooling
implementation is listed below.
Evidence is code and synthetic tests, not hardware acceptance. A successful
backend call does not prove physical fan speed or cooling capacity.

| Claim | Backend | Evidence (test/code) | Not claimed |
|---|---|---|---|
| Normal requested fan floors are 40 % (or a higher backend minimum); pump protection requests 100 %; unrestricted settings can bypass protection | Linux hwmon, GPU, Windows motherboard | `hwmon.rs` `manual_assignment_enforces_floor_and_release_restores_firmware`, `refuses_headers_owned_by_another_tool_and_pumps_stay_full`; `NativeSessionTests.cs` (pump reduction rejected) | Correct header-to-fan mapping or actual speed; GPU pump identification |
| Release, pause, normal shutdown and faults attempt to restore the recorded firmware mode | Linux hwmon | `hwmon.rs` `manual_assignment_enforces_floor_and_release_restores_firmware`, `firmware_takeback_and_stall_fault_return_to_firmware`, `every_driver_restores_its_own_enable_semantics` | Successful restoration when sysfs writes fail; sensor loss requests full duty rather than firmware return |
| Release, pause, shutdown and write faults attempt driver automatic mode | GPU (NVML, ADLX) | `gpu_cooling.rs` `take_set_and_release_return_to_driver_auto`, `write_fault_releases_and_records`, `shutdown_and_pause_release_and_crash_journal_recovers` | Successful return when the driver fails; sensor loss requests full duty rather than driver automatic mode |
| After a daemon crash or kill, recovery attempts journaled channels when freeRGB **next starts**; failed returns remain journaled | Linux hwmon, GPU | `hwmon.rs` `drop_restores_and_saved_assignments_resume_after_crash_recovery`; `gpu_cooling.rs` `shutdown_and_pause_release_and_crash_journal_recovers` | Successful return for missing devices or failed writes; any recovery while freeRGB stays stopped, or after OS crash, kernel crash or power loss |
| Missing or stale temperature, or the critical temperature, requests 100 % | Linux hwmon and GPU curves; Windows native telemetry guard | `docs/COOLING_LINUX_HWMON.md`; `gpu_cooling.rs` header (`fail-safe composite output means 100 %`); `NativeSessionTests.cs` (emergency latches 100 %) | A working sensor; that 100 % is actually applied by the hardware |
| On a fault a **verified** Windows channel attempts 100 % and keeps its reservation; it does **not** return to firmware until an authenticated supervisor resolves it | Windows motherboard, verified tier | `NativeSessionTests.cs` (transport does not silently release after emergency), `NativeTierTests.cs` (verified fault behaviour) | Firmware return for this tier |
| On a fault a lower-tier Windows channel attempts a return to the firmware mode captured in its worker context | Windows motherboard, lower tiers | `NativeTierTests.cs` (tiered lease expiry ends ownership; failed return keeps intent) | A verified readback on every chip; any recovery after process, OS or power failure |
| Manual mode requests 100 % for missing, invalid, stale or critical temperature and after a loop gap, when evaluation runs | Linux hwmon, GPU | `manual_interlock.rs` (85 C critical, 5 C hysteresis, 3000 ms sensor age/loop gap); `hwmon.rs` `manual_without_a_fresh_temperature_goes_full`; `gpu_cooling.rs` `manual_interlock_forces_full_duty_without_a_trusted_sensor` | Execution while the daemon or driver is hung; GPU production backends currently provide no Manual temperature, so request 100 % |
| Manual acquisition requires a durable journal; failed restoration remains unresolved and blocks reacquisition | Linux hwmon, GPU | `hwmon.rs` `unavailable_journal_blocks_first_write`, `failed_restore_stays_journaled_until_proven`, `unreadable_journal_blocks_provider_and_is_kept`; `legal_audit_gpu_recovery.rs` | Successful recovery despite inaccessible hardware; control without a state directory |
| Lease policy and guard process fall back to a **synthetic** 100 % request | Test fixture only | `cooling_guard_process` test (see below) | Any real fan; production emergency system |

Not promised without proof: recovery after a crash or kill of the process,
service, kernel or machine; recovery while the daemon stays stopped; recovery
when a driver, firmware or the hardware ignores or reverses commands; any
guarantee that a fan reaches a given speed. The Windows broker and Linux hwmon
paths have not been hardware-accepted on most boards (see
`docs/promts/fancontrol/07-hardware-acceptance.md`).

## Unsafe code

The workspace default is `unsafe_code = "forbid"`. Exceptions with FFI or OS
calls: `freergb-gpu-nvml` (NVML loaded from the driver at run time),
`freergb-gpu-adlx`, `freergb-storage-temp`, `freergb-tray`, the desktop memory
helper, `freergb-gui-win32` and `freergb-update` (explicit Windows installer
elevation). These crates use `deny` or `allow` with scoped
`#[allow(unsafe_code)]`. The workspace-wide "forbid" statement therefore does not
describe these crates.

This increment adds reusable, hardware-independent policies in
`freergb-control::{calibration,cooling_safety}`. The calibration and lease policies do not themselves provide production
motherboard access. Neither policy can write to a fan, establish its physical
identity, select PWM/DC or prove that a requested speed was applied.

## Bounded fan response experiment

`Calibration` starts with sequence 1 requesting 100%, then tests 80%, 60% and
40%. Pumps are rejected. These bounds deliberately do not measure fan-stop or
minimum startup duty; a real backend must never label their result as that.

Each command needs the matching applied sequence within 1000 ms of issue. This
acknowledgment must eventually come from a checked backend operation, not merely
an IPC enqueue. After acknowledgment, wait at least 2000 ms, then evaluate a rolling
window of five RPM readings whose acquisition times are at least 250 ms apart.
Only a stable window becomes the stage's samples; a ramp keeps settling. Re-reading
the same cached acquisition does not count again. The first stage allows 15000 ms
after acknowledgment for spin-up from firmware duty; later stages allow 10000 ms.
Expiry requests restoration with `unstable_or_missing_samples`. The session budget
is at least 60000 ms and covers all stages plus up to nine restart probes at their
deadlines, including acknowledgment time. Runtime callers must keep polling;
the model itself owns no thread or hardware handle.

RPM must be finite and 100..100000; temperature finite and -20..150 Celsius.
Missing/stalled/out-of-range input, either sample older than 1500 ms, future
sample time, critical temperature, unexpected future acknowledgment, missing/late
acknowledgment, reversed clock, time limits or cancellation request 100%.
Critical temperature is caller-specified, finite, 40..100 Celsius. At least a
100-RPM lower bound is an experiment prerequisite, not a universal fan/pump limit.

Five readings are stable only if minimum >=97% of maximum and a monotonic trend
changes by at most 0.5% of maximum across the window. A stall sweep requires all
five readings to be zero before recording a stall. A lower duty stage
whose mean RPM rises >10% over the preceding stage aborts. At the final stage,
mean RPM <=90% of the initial mean marks `response_observed:true`. This is a
response indication, not proof of wiring: temperature/load/firmware changes can
also change RPM. Only that response gives a provisional 50% suggested floor
(40% tested +10 percentage points). This is not an experimentally established
startup threshold, a guaranteed safe operating floor or an automatically applied
setting. Constant RPM produces no floor suggestion.

The final 100% restoration command also requires acknowledgment within 1000 ms.
Sensor faults during restoration suppress a successful experiment result without
resetting or extending that acknowledgment deadline.
Phases: `applying`, `settling`, `sampling`, `restoring`, `completed`, `aborted`,
`restoration_failed`. Results become available only after successful restoration;
failed restoration never exposes a successful result. The requested duty remains
100% on terminal states. `hardware_verified` is always false. A future backend
must report actual write/readback errors and retain independent recovery if its
writer blocks, disappears, loses access or cannot acknowledge restoration.

## Preview API

`cooling.calibration.preview` is stateless and writes no configuration. It takes
`{role:"fan",critical_c:85,samples:[...]}`. Each sample contains:

```json
{"at_ms":0,"applied_sequence":1,"rpm":2000,"rpm_age_ms":0,"temperature_c":40,"temperature_age_ms":0}
```

All times are milliseconds relative to the preview's initial time 0. RPM,
temperature and acknowledgment may be null/omitted, representing missing values.
`at_ms`, both ages, role, critical temperature and sample array are required.
At most 512 samples, each `at_ms` in 0..61001; unknown fields are ignored.
Reversed sample times exercise a clock fault rather than being sorted/repaired.
Out-of-range/missing sensor values exercise the fallback, not an RPC format error.
For a full synthetic run, acknowledge every newly returned sequence promptly,
hold each requested duty for settling/sampling and acknowledge the final 100%.

Returns `{initial,trace,final_output,simulated:true,hardware_control:false}`;
each output has `phase`, `sequence`, `duty_percent`, `reason:string|null` and
`result:null|{response_observed,lowest_tested_percent,suggested_floor_percent,
hardware_verified:false}`. One trace output per supplied sample. An empty array
returns the initial 100% request. Truncated traces remain incomplete; they do not
cause background jobs or hardware actions. Invalid format/limits, unsupported role
or critical temperature return existing `-32602`. No API version/config changes.

## Independent lease enforcement policy

`LeaseGuard` has its own monotonic clock and retains a low duty only while a valid
command lease is active. Constructor bounds: 500..3000-ms lease, 20..100% floor;
initial request is 100%. A pump can be represented with a 100% floor, independent
of the fan-only experiment. Each command needs a strictly increasing sequence,
nondecreasing issue time, no future issue time, age <=250 ms and duty within
floor..100. Renewal expires at **issue time + lease**, not receipt time, so
buffered/replayed commands cannot extend low duty. At expiry, stop/EOF or clock
reversal, fallback is latched at 100%; later commands cannot revive the session.
A fresh authenticated session is required. Invalid commands are rejected without
extending the lease; a transport may additionally stop on rejection.
Lease renewal sequence numbers are separate from calibration's applied-step
sequence: heartbeats renew the current duty even while a calibration stage waits
for settling/samples. A renewal alone is not a new applied-step acknowledgment.

This policy does not authenticate users, map clocks across processes, supervise
physical RPM/temperature or perform I/O. A future privileged runtime needs protected
ownership, authenticated sessions/clock origin, bounded commands, an independent
sensor/stall watchdog and a hardware-capable recovery path which remains usable
if the writer hangs. Native locks/driver hangs, OS/service crash, suspend/resume,
power loss, firmware restoration and fallback write failures remain unresolved.
These policy models are hardware independent; no real fan receives their
fallback. Real backends have their own, separately described fault handling
(see "Safety claims by backend").

## Real process tests with synthetic duties

`cargo test -p freergb-daemon --features cooling-fixture --test cooling_guard_process --locked`
builds the explicitly feature-gated `freergb-cooling-guard-fixture`. It has no
hardware backend and is excluded from normal builds and the installer. A separate
producer process requests 40%; tests then leave it hung or kill it. The independent
guard process emits a simulated 100% fallback on timeout or EOF. Oversized input
without a newline is bounded and stops the session. Its reader thread cannot
block the guard's polling loop. These run on both Windows and Linux CI and Package.

The tests prove independent process scheduling/lifetime and synthetic fallback
decisions only. They do not prove a real register write, physical fan speed,
survival of a failed guard process or an operational production emergency system.
