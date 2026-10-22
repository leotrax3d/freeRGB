# Private native-session candidate

`NativeControlSession` and `NativeSessionTransport` connect an owned native port
to a bounded private worker pipe and lease watchdog. They are compiled into the
helper and called by the protected dedicated worker. The snapshot service stays
read-only; public cooling.sessions RPC exposes explicit experimental starts with UAC.
Installer opt-in alone does not authorize control; shared config formats are unchanged.

## Trust boundary

`RunProtected` requires the fixed protected SYSTEM helper, a connected pipe owned
by SYSTEM and a client PID matching the registered running own-process LocalSystem
service with its exact protected image configuration. The peer process token must
also be SYSTEM; SCM PID equality alone is insufficient against reuse. No native
acquisition occurs until those checks pass. Test constructors/runners exist only
under `NATIVE_CONTROL_TESTS`; production gate tests assert they are absent.

The supplied port is already tied to a single explicitly selected control object.
Messages contain no device selector, executable/library path, register or shell
argument. Future activation still needs verified channel selection and a protected
service-to-worker pipe lifecycle; these factories are not an external listener.

The subsequent journal bridge is mandatory in the production session/transport
factory signatures. It records intent before acquisition and retains unresolved
state across worker death; see MAINBOARD_CONTROL_RECOVERY.md. No production caller
activates the chain yet.

The service-side `NativeSessionClient` is also a candidate with no production
caller. Its factory requires the protected SYSTEM service, a SYSTEM-owned pipe
and the exact expected worker PID/protected helper image. Merely matching another
copy of the helper executable is insufficient. The SCM fixture exercises the same
worker-identity check in its private authentication probe.

## Private version-1 exchange

Each server reply has `version:1`, random 128-bit `session` and `ticket` hex strings,
last accepted `sequence:int`, `state`, nullable `reason`, `duty_percent`,
`ticket_ms:250`, `lease_ms:1000`, `owned:bool`, `physical_write_verified:false`.
Tokens travel only on the private pipe, not logs or public RPC. A reply is a state
snapshot; clients must not assume its ticket remains fresh after a delayed read.

One request/response at a time. Set request:

```json
{"session":"<server token>","ticket":"<last server ticket>","sequence":1,"action":"set","duty_percent":40}
```

Close omits `duty_percent` and uses `action:"close"`. Exact field count is checked;
unknown fields, wrong types/identities, counter jumps/replays and duties outside
the port's fan40..100/pump100 range latch a 100% request. Sequence must equal the
prior accepted counter +1, bounded by Int32.MaxValue. The private protocol does
not accept fractional counters/duties or client-supplied clocks. Requests are UTF-8
JSON lines, at most 2048 bytes including LF, bounded nesting; extra buffered data
after a newline is rejected. There is no request pipelining.

The typed service client validates version, token shape/session continuity,
counter, requested duty, ownership/state consistency and unverified-write flag
on replies. Active replies require a fresh ticket. It closes the pipe on protocol/
I/O/local ticket faults rather than reconnecting or replaying commands. Its local
200-ms send window is conservative; the worker's ticket issue time remains the
authority, including when a reply was delayed in transit. No automatic heartbeat
or listener has been enabled. Terminal receipts cannot be used for another set
or close; disposal alone does not request firmware release.

A ticket is single-use and accepted only within 250 ms of **server issue time**.
The active duty lease expires at that issue time +1000 ms, never receipt time.
After a successful callback, the server mints the next ticket using its monotonic
clock. Heartbeats therefore need fresh tickets and repeated duties; a caller
cannot buffer a stream of renewals in advance. A callback completing after its
lease deadline triggers fallback rather than minting another ticket.

## Fallback and ownership

Initial acquisition requests 100%. States: `active`, `emergency`,
`recovery_failed`, `closed`. A dedicated 25-ms polling thread remains separate
from the pipe's bounded 2000-ms input wait and 2000-ms write wait. Lease expiry,
clock reversal, EOF, transport failure or invalid requests latch an emergency
100% request. This is not silently released: the supervisor retains responsibility
for the owned channel and must resolve restoration/recovery explicitly. Later
messages cannot revive an emergency session. If the fallback callback fails, the
state is `recovery_failed`; the response does not claim a successful physical write.

A valid timely close requests 100%, then selected-channel firmware restoration;
only successful callback completion reports `closed`. Failed restoration is
reported rather than releasing the reservation. Tokens do not authorize the
supervisor to ignore physical verification or a failed recovery state.

The watchdog is independent of blocked **pipe input**, not of a hung native call:
the session/native object locks still serialize I/O. A stalled driver/callback can
block fallback, and process death loses in-memory recovery state. Production
uses the protected crash-recovery journal, worker lifetime identity and checked
recovery/readback path. A returned callback alone is not physical hardware
acknowledgment. Limited CHA_FAN2 results are recorded in COOLING_ACCEPTANCE.md;
driver hangs and broader fault acceptance remain open.

## Evidence

`test-mainboard-control.ps1` covers token/sequence/shape rejection, expiry based
on ticket issue time, clock faults, late completed writes, pump bounds, failed
fallback and explicit close. Real Windows pipes test input stall, disconnect and
oversized unterminated records, with the watchdog acting before the input timeout.
One test couples the lease to the actual pinned LHM Control adapter with synthetic
events, observing 100 ->40 ->100. No Computer/SuperIO/driver is opened.

SCM fixture builds add a separate SYSTEM-only probe pipe. The actual registered
service connects to its protected worker, which verifies service identity/token;
subsequent synthetic snapshots report `service_client_authenticated`. Standard-user
read/upgrade/rollback/crash tests require that evidence. These probes are compiled
out of production. They run only on disposable GitHub Windows service acceptance;
local tests do not install a service/driver. This proof is distinct from a live
fan-control session and must not be presented as hardware acceptance.

Typed-client real-pipe tests additionally cover normal set/close, cancellation,
an expired local ticket and altered session/counter/duty replies. Rejected replies
close the pipe and cause the synthetic worker to request full speed. Both PS7 and
PS5.1 acceptance pass; no session secrets are written to test output/logs.

## Telemetry interlock candidate (2026-10-21)

The inactive protected session factory now requires a telemetry guard and the
same monotonic clock as its trusted local sampler. Before acquisition it requires
fresh temperature/RPM. Session polling and commands both check the guard, so
valid command traffic cannot hide lost sensor updates. Samples older than 1500 ms,
clock reversal, repeated/future acquisition timestamps, non-finite or out-of-range
values, temperature at the configured 40..100 C limit, or RPM below 100 for 1000 ms
latch a fault. Critical heat latches on observation even between polls. Faults
request 100% and retain the recovery reservation; later good samples cannot revive
the session. This is a conservative no-fan-stop pilot policy, not calibration.

This is a cached interlock, not a sensor reader or physical write verifier. No
production sampler constructs it yet. The sampler must provide acquisition times,
not refresh cached readings with new receipt times; selected sensor identity and
actual hardware readback remain prerequisites. Locks/native calls may still block.
The 25 ms transport watchdog is independent of pipe input, not native driver hangs.

Architectural follow-up: keep reusable calibration, curve and supervision policy
in Rust (`freergb-control`/daemon); restrict C# to the necessary LHM/native boundary.
No public API, GUI/config format or live-control enablement changed.

## Rust peer (2026-10-21)

`freergb-daemon::native_session` implements an async peer for the same private
version-1 worker protocol using existing Tokio/Serde dependencies. It accepts an
already authenticated transport; it does **not** authenticate a pipe or launch a
worker itself and has no production caller/listener. The eventual protected Rust
supervisor must validate SYSTEM/service/worker lifetime before construction.

Replies are bounded to 2048 bytes including LF and two seconds. Writes/flush have
a two-second bound. Initial reply must be active/owned/100%, counter zero; receipts
must match session, requested counter/duty, new ticket and ownership semantics.
Ticket/lease parameters remain 250/1000 ms, with a conservative local 200 ms send
window. There is no auto-reconnect/replay/heartbeat. Invalid or cancelled operations
drop the transport and make that client unusable. Server-side lease/telemetry must
continue independently; an async timeout cannot resolve a hung native driver.

Secret-bearing reply/client types deliberately have no Debug output. Public Rust
receipts contain callback state/duty/ownership/reason and optional native-return
evidence, never tickets. Additive private reply fields `firmware_return_verified`
and `journal_cleared` default to false for older workers and are rejected as true
unless the session is closed. The production journaled adapter asserts them only
after checked native firmware return and durable intent clearance respectively.
Mechanical/RPM verification remains separate; `physical_write_verified` is still
false. A closed receipt alone cannot authorize maintenance.
See `COOLING_RUNTIME.md` for the Rust runtime/owned host and remaining integration.
This is an internal Rust module, not a new JSON-RPC API or
shared configuration format. The old C# peer remains until authenticated Rust
hosting is integrated and accepted.

Duplex-stream tests exercise set/close, changed session/counter/ticket/duty,
unjustified physical verification, oversized nonterminated input and cancellation
during a pending receipt. These use a synthetic peer, not LHM or a real driver.

## Cross-language pipe acceptance

`scripts/test-mainboard-interop.ps1` builds the test-only C# executable then runs
the explicitly ignored Windows integration tests in `native_session_interop.rs`.
The script selects the test executable through a temporary environment variable;
it is not a daemon configuration or activation mechanism. Windows CI invokes the
same tests after building the pinned native test fixture.

Rust uses a real Tokio Windows named-pipe client against the actual C# session
and `NativeSessionTransport`, with `SessionPort` as a memory-only native sink.
Assertions inspect exact synthetic write order: acquire100, duty40, close100,
release; disconnect or a silent connected client instead requests100 and retains
ownership in emergency. Fixture output contains no tickets and always reports
simulated/physical_write_verified=false. No LHM assembly is loaded in this mode.

The test pipe grants the current test user full access, modelling the privileged
private SYSTEM peer's rights. It is **not** the public sensor pipe's restricted
ACL. Tokio's default generic-write open does not work against that public ACL;
production permissions are deliberately unchanged. These tests do not establish
SYSTEM authentication, registered-service identity or worker-lifetime binding for
the Rust client; those remain required before production use. The C# fixture mode
exists only in the test executable and never enters installer payloads.

Current integration and hardware/release evidence: [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md).
