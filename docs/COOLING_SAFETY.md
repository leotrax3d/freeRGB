# Cooling calibration and lease policy

This increment adds reusable, hardware-independent policies in
`freergb-control::{calibration,cooling_safety}`. Production motherboard access
remains read-only. Neither policy can write to a fan, establish its physical
identity, select PWM/DC or prove that a requested speed was applied.

## Bounded fan response experiment

`Calibration` starts with sequence 1 requesting 100%, then tests 80%, 60% and
40%. Pumps are rejected. These bounds deliberately do not measure fan-stop or
minimum startup duty; a real backend must never label their result as that.

Each command needs the matching applied sequence within 1000 ms of issue. This
acknowledgment must eventually come from a checked backend operation, not merely
an IPC enqueue. After acknowledgment, wait 2000 ms, then collect five RPM samples
whose acquisition times are at least 250 ms apart and after settling. Re-reading
the same cached acquisition does not count again. A stage is limited to 10000 ms
after acknowledgment and a session to 60000 ms. Runtime callers must keep polling;
the model itself owns no thread or hardware handle.

RPM must be finite and 100..100000; temperature finite and -20..150 Celsius.
Missing/stalled/out-of-range input, either sample older than 1500 ms, future
sample time, critical temperature, unexpected future acknowledgment, missing/late
acknowledgment, reversed clock, time limits or cancellation request 100%.
Critical temperature is caller-specified, finite, 40..100 Celsius. At least a
100-RPM lower bound is an experiment prerequisite, not a universal fan/pump limit.

Five samples are stable only if minimum >=80% of maximum. A lower duty stage
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
No actual fan receives this fallback in the current product.

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
