# Private native-session candidate

`NativeControlSession` and `NativeSessionTransport` connect an owned native port
to a bounded private worker pipe and lease watchdog. They are compiled into the
helper but have **no production caller/listener/CLI activation**. Snapshot service,
public JSON-RPC and installer opt-in remain read-only. No GUI/config format changes.

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
activation remains blocked on a protected crash-recovery journal, worker lifetime
and independently usable recovery/readback path. A returned callback is not
physical hardware acknowledgment. No physical fan is currently driven by this
transport.

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
