# Rust cooling execution and hardware acceptance

Updated 2026-10-22. This documents implemented execution code and its remaining
production boundaries. It does not announce enabled live fan control.

Latest acceptance: `MAINBOARD_CONTROL_BRIDGE.md` documents the now-connected
protected service/native sampler and successful actual 80% Rust curve/calibration
pilot on CHA_FAN2. The additive session API and Classic dialog are implemented; interactive public-path acceptance remains.

## Rust execution

`freergb-daemon::cooling_runtime::run` now executes either a validated fan curve or
the full existing calibration state machine through `NativeSessionClient`.
It accepts an already authenticated native session, an initial trusted sampler
frame, a bounded sampler receiver, a stop channel and a shared monotonic epoch.
The protected host must bind real sensor IDs to the selected channel before
constructing this execution. GUI-supplied numbers are never trusted sampler data.

The runtime checks initial fresh temperature/RPM and a running fan. It uses the
existing Rust telemetry/lease policy; unchanged cached timestamps cannot renew
freshness. Curve evaluation and hysteresis use `freergb-control`; native pilot
curve minimum is at least 40%, critical temperature 40..100 C. Automatic calibration
is fan-only and retains its 100/80/60/40 sequence, bounded sampling and final100
acknowledgement. Existing saved curve formats/ranges are unchanged; unsupported
runtime limits reject execution rather than silently rewriting saved curves.

Native sessions use a seven-second acquisition-age ceiling in the Rust telemetry
guard, native receipt decoder and C# native guard (owner R5b, 2026-11-06).
Acquisition starts before I/O: consecutive three-second reads and the native
100 ms sampler pause can make the last published reading over six seconds old.
Other Rust samplers retain their 1500 ms default. The broker inventory reader is
separate (one-second pause plus snapshot time; five-second cache expiry) and does
not feed live native sessions. Ready inventory alone cannot validate live timing.

Linear/curve sessions still request 100% at temperature age >3000 ms, bypassing
shaping/rate limits, and immediately on observed critical heat. At age >7000 ms,
native supervision latches stale and ends the session, with explicit firmware
return when supported and existing owned-full-speed recovery otherwise. Sample
identities/acquisition times never advance merely because tickets or command
receipts arrive. Command leases, receipt deadlines, stall checks, floors and
recovery evidence requirements are unchanged. Calibration retains its stricter
1500 ms measurement-age requirement; this change targets live-session timing.

Native requests run in a separate Tokio task behind a one-slot queue. The control
loop polls every25ms, refreshes a healthy native session after100ms and imposes a
150ms pending-receipt deadline. Sensor updates, stop and watchdog checks continue
while native I/O awaits a response. Dropping/cancelling the runtime aborts its
native task and closes the transport; the native worker must independently handle
EOF/lease expiry. A hung kernel/driver is not solved by cancelling an async task.

Explicit stop and normal calibration completion request native close, which first
requests100 then returns control. A native closed callback is reported separately
from maintenance authorization. The latter requires BOTH authenticated
`firmware_return_verified` and `journal_cleared`. Missing fields from an older
worker defaultfalse. Errors preserve recovery intent; no reconnect/replay or old
low duty is resumed. A commissioning result still has hardware_verified=false;
observed RPM response alone does not authorize arbitrary channels automatically.

## Native boundary

C# retains only the LHM/native boundary and existing protected transport/journal.
The production `NativeControl` factory now requires register readback from the
pinned LHM `Nct677X` implementation, an initialized ISA mutex and an audited
separate-mode-register chip scheme. Shared-bit/EC NCT668x variants and other
unimplemented evidence backends are refused rather than assumed compatible.

Before acquisition, it captures chip/index/port/revision/original automatic mode
and command into optional journal recovery metadata. A channel already in manual
mode is refused because foreign ownership cannot be established. Every write
requires actual native mode/command register agreement after the callback; every
release requires original firmware-mode readback before ownership/journal clear.
The automatic firmware may legitimately change its command byte after return.
This verifies the command/mode registers, not a connected motor's RPM response.

This is necessary because pinned LHM setters can return silently on bus-lock
contention: cached SoftwareValue/ControlMode or successful callback return alone
cannot establish a hardware write. Source basis:
[Nct677X](https://raw.githubusercontent.com/LibreHardwareMonitor/LibreHardwareMonitor/v0.9.6/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc/Nct677X.cs),
[Mutexes](https://raw.githubusercontent.com/LibreHardwareMonitor/LibreHardwareMonitor/v0.9.6/LibreHardwareMonitorLib/Hardware/Mutexes.cs).
LHM's null-mutex success path is also rejected by the evidence constructor.

Protected recovery checks chip/index/port/revision plus recorded worker/service
lifetimes. A dead worker's original baseline is restored only under the same
registered service lifetime; an already returned original mode permits a
non-writing clear. A living foreign worker, missing provenance, changed service
lifetime with differing native mode, or failed readback retains unresolved intent.
No old low duty is replayed. See `MAINBOARD_CONTROL_RECOVERY.md`.

## Owned execution and maintenance

`freergb-daemon::cooling_host::CoolingHost` reserves admission before invoking an
authenticated acquisition factory. It owns the async acquisition/runtime task,
permits only one channel (matching today's native journal), and bounds acquisition
to 500ms. It does not supply OS authentication or commission a channel itself.

Maintenance closes admission before sending the stop request. The host waits for
the runtime's explicit close and requires both firmware-return and durable-clear
evidence. Missing proof, acquisition failure, task loss or timeout blocks further
admission and replacement. Dropping execution aborts the task and closes transport;
it never treats EOF as successful release. Ready is terminal. The elevated
installer must still hold its existing shared native reservation mutex and use a
production service handshake, now implemented with independent native drain.
Same-version replacement passed; release acceptance remains open.

Tests execute the actual runtime through duplex native-protocol peers: verified
drain succeeds, missing proof and stuck acquisition block. Separate runtime tests
keep successful command receipts flowing while temperature goes stale or RPM
reports a stopped fan. Critical-temperature processing also continues while a
native acknowledgement is stuck. These are synthetic measurements, not hardware
fault acceptance.

## Actual local inspection (authorized)

The user explicitly authorized installation of the protected mainboard service.
The existing opt-in bootstrap installed `freeRGBMainboard` under Program Files
and reused the already running PawnIO driver. No new driver was installed.
SignalRGB/launcher processes were closed under the earlier authorization.
The user-level helper successfully read the protected service.

The ASUS PRIME X870-P WIFI reports Nuvoton NCT6701D, seven control candidates,
temperature channels, and only one nonzero RPM reading (Fan#2, approximately1700).
Raw local evidence is in `target/mainboard-live-snapshot.json` (not committed).
The user confirmed that all fans/AIO are Corsair iCUE LINK devices and only the
AIO has a cable to the motherboard. The detected RPM is therefore not evidence
of a motor powered/controlled by that motherboard header. Exact cable/header
identity has not been electrically verified. That initial inspection changed no duty.

A real directly motherboard-powered fan is required to finish physical response,
fault and restoration acceptance. The user subsequently connected a normal fan
to CHA_FAN2. A second actual snapshot now shows a new RPM source
`/lpc/nct6701d/0/fan/2` around1240RPM; that identifies a candidate tachometer by
the physical change. Evidence is in `target/mainboard-live-with-fan.json`.

Subsequent administrator-authorized, fixed100 diagnostics on control/2 verified
the command register, observed roughly1324 ->1709RPM, restored the original
firmware mode, then cleared the journal. A later snapshot returned around1300RPM.
A separate test killed the exact lifetime/path/parent-matched protected worker
during that100% diagnostic: the unchanged service supervisor restarted it,
protected baseline recovery succeeded, and the journal was cleared. The probe's
EOF/fault result in that crash test is expected, not a successful probe receipt.
Subsequent 80% curves and calibration ran on the selected fan; sustained/public-path acceptance remains open.
A telemetry-only AIO connection must never be
accepted as a successfully controlled motherboard fan. The six current explicit
interop tests include full Rust calibration against constant synthetic RPM and
require response_observed=false, no floor recommendation, hardware_verified=false.
They also cover Rust curve execution/return and sampler-loss recovery ownership
over the actual C# protocol on a current-user test pipe, without hardware access.

## Remaining acceptance

The production Rust host, authenticated native sampler/transport, additive session
API, Classic dialog and protected service maintenance handshake are implemented.
Same-version active replacement passed on CHA_FAN2. Remaining gates:

- Interactive public API/Classic UAC and verified-return acceptance.
- Broader service/boot transitions and driver/native-hang acceptance.
- Release-to-release upgrade, rollback, actual uninstall and maintenance failures.
- Wider fan/chip and RGB-controller hardware acceptance.

## Protected diagnostic files

The administrator-only developer script `scripts/probe-mainboard-100.ps1` requires
`-ConsentToProbe` and a selected `-ControlId`; it is not a normal end-user setter.
Its optional `-CrashWorker` test never requests reduced duty. Fixtures compile out
the hardware diagnostic. Under `%ProgramFiles%/freeRGB`, the protected one-use
`mainboard-probe-request.json` has exactly `version:1`, `id` (GUID N), `control_id`
(bounded native ID), and `expires_utc` (UTC round-trip timestamp, at most30s ahead).
The SYSTEM service consumes the request before dispatch. Service command128 alone
does not authorize a write without this protected request. A fixed
`mainboard-probe-result.json` contains `version:1`, matching `id`, and `result`
(the worker result, including verification flags/readings or a fault). These are
protected diagnostic artifacts, not GUI settings or a persistent control opt-in.

Current integration and hardware/release evidence: [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md).

## Support tiers in the Rust runtime (2026-10-22)

`run_with_limits` takes `RuntimeLimits {floor_percent, allow_stop}` derived from
`freergb_control::fan_support::limits`; `run` keeps the legacy 40% floor. Fan stop
(`allow_stop`) requires a worker that admitted `unrestricted`; it relaxes the
running-fan start check and makes a stalled tachometer a warning. For lower tiers
a supervision fault no longer drops the transport: the runtime sends `close` (100%,
then the worker's owned firmware return) and reports `fault`; a worker-side fault
return is accepted as such. Verified workers keep the previous hold-100%/recovery
behaviour. Reports add `tier`, `return_evidence`, `fault`, `warning`.

## CPU/GPU and mixed temperature sources (2026-10-23)

Curves may use CPU (`/amdcpu/…`, `/intelcpu/…`) and GPU (`/gpu-…`) temperatures
in addition to SuperIO sensors; up to three sources can be combined and the
hottest one drives the curve (FanControl-style "max" mix). The protected service
now enables LHM's CPU and GPU groups (read-only). One SuperIO source keeps the
original checked path; other sources are refreshed through their own LHM
hardware inside the worker, and any missing or unreadable source stops the
fresh samples so the independent guard returns the channel. The RPM sensor must
stay on the control's chip. Validation: `NativeSources` (C#) and
`cooling_sessions::temperature_sources` (Rust).

## Multiple channels (2026-10-24)

Up to 16 channels run at once (`MAX_ACTIVE`); each is its own task with its own
native worker, lease, watchdog and firmware return, and the daemon command queue is
bounded. A sensor, lease or worker fault ends only that channel (100 % / BIOS
return as before). Because each channel has its own worker process, a worker or
transport failure returns that worker's channel; the protected journal (v2, up to
16 reservations, v1 still read and written for one channel) lets recovery return
every orphaned channel after a crash. Daemon shutdown and `stop_all` return all.

API (additive): `cooling.sessions.list`, `cooling.sessions.stop_channel
{control_id}`, `cooling.sessions.stop_all`. `start/stop/status` are unchanged.

`cooling-assignments.json` remembers curve and fixed-speed channels with their
hardware identity (chip device id, native control, saved channel binding). At
daemon start they resume through the normal policy only if the identity is
unchanged; otherwise they are listed as `needs_attention` and not started. An
explicit stop forgets the assignment. Windows still authorizes each resumed
channel through the protected helper; the GUI asks the user only once per app
session.

Safety mechanisms are unchanged: watchdog, lease expiry, journal recovery, tier
floors, pumps only at 100 %, BIOS return.

### Hardware validation

Real hardware: same-version replacement on CHA_FAN2 (single channel, see above).
Concurrent multi-channel operation is verified against mocks and synthetic C#
journal tests only. Please report results with the "Hardware report" issue template.

## Composite curves in live sessions (2026-10-24)

`ControlMode::Composite { output, critical_c, feed }` takes its duty from the
daemon's `CompositeFeed`, which `Controls::refresh_sensors` republishes every
second from the saved composite curve set (`freergb_control::curves`). Missing,
stale (> 3 s) or fail-safe outputs, a stale session temperature or reaching
`critical_c` on the session's own temperature give 100%; the duty is clamped to
the tier floor. The native worker sees only duties and keeps its lease
supervision, so a stopped daemon ends in the same fault path as any other live
session. Covered by `composite_mode_follows_the_feed_and_fails_safe` (simulated
worker); no hardware acceptance yet.
