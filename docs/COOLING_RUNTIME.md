# Rust cooling execution and hardware acceptance

Updated 2026-10-21. This documents implemented execution code and its remaining
production boundaries. It does not announce enabled live fan control.

Latest acceptance: `MAINBOARD_CONTROL_BRIDGE.md` documents the now-connected
protected service/native sampler and successful actual constant100% Rust curve
pilot on CHA_FAN2. The normal daemon/API/GUI and variable-duty paths remain gated.

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
  production maintenance handshake; its commissioning transport now reaches the
  protected service, but installer draining remains unwired.

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
No reduced-duty test or productive curve acceptance has been performed.
A telemetry-only AIO connection must never be
accepted as a successfully controlled motherboard fan. The six current explicit
interop tests include full Rust calibration against constant synthetic RPM and
require response_observed=false, no floor recommendation, hardware_verified=false.
They also cover Rust curve execution/return and sampler-loss recovery ownership
over the actual C# protocol on a current-user test pipe, without hardware access.

## Remaining before public activation

- A production protected Rust host and authenticated sampler/native transport
  startup wired into the real Windows service; public service requests remain
  read-only, with a separate protected administrator fixed100 diagnostic.
- Broader service/boot transition and driver/native-hang acceptance.
- Service maintenance request/return acknowledgement tied to its global admission
  lock, multiple channel reservations, and active-session installer acceptance.
- Actual powered-fan and RGB-controller acceptance on supported hardware.
- Only after that: additive documented daemon methods and both GUI integrations.

No public API/config file changed in this increment. The Tauri branch is untouched.
Motherboard fan power and RGB/ARGB are separate backends. This work does not add
universal RGB support or guarantee every manufacturer/chip; missing protocols must
remain explicitly unsupported until their adapters are implemented and validated.

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
