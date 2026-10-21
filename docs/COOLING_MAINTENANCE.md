# Cooling maintenance admission policy

`freergb-control::maintenance` is a pure Rust state machine for the protected
supervisor. Its active-session release handshake is not yet called by the
installer or a production control service. The existing Windows maintenance entry
now implements the conservative journal exclusion described below. Live control
remains disabled. No API or configuration format changes.

## Production refusal and exclusion (2026-10-21)

The existing elevated broker install/remove entry now takes the **same global
mutex as native journal reservation**, validates the journal and holds that mutex
through install/update/rollback/removal. Pending, corrupt, unsupported or oversized
state refuses the operation before staging payloads, stopping the service or
removing files. Lock contention times out and refuses acquisition/maintenance.
The separate preexisting deployment file lock still serializes maintenance calls.

The mutex may initially be created by either the SYSTEM worker or elevated
administrator maintenance. Owner must be SYSTEM or Administrators, and only
those identities may hold allowed access. Maintenance can run from its verified
staging executable under elevation; it does not require a running SYSTEM service.
Its factory uses the fixed protected parent, not a supplied journal path.

This deliberately does not auto-clear an intent or stop a live controlled worker.
If a future session is active, its pending intent blocks maintenance. The future
supervisor handshake must first obtain verified return and preserve intent until
that verification. A mutex with a clear journal excludes new reservations while
maintenance runs; after process death, an abandoned mutex still requires journal
validation. This is not a complete supervisor lifetime or driver-hang solution.

Local synthetic tests cover refusal and a concurrent acquisition waiting until
maintenance releases the shared mutex. Disposable SCM CI additionally attempts
upgrade and removal with corrupt/pending state and requires unchanged service PID,
executable hash and journal bytes. Check that run's result separately; local tests
never register the service or access hardware.

## Lifecycle

The supervisor initializes the gate only after checking recovery state. Missing
recovery verification, a corrupt/unsupported journal or unresolved channel must
initialize it blocked. Initialization with `recovery_clear=true` is a trusted
supervisor decision, never a client parameter.

Running admits at most 128 channel sessions, each assigned a monotonically
increasing gate-local token. Register before native acquisition; failed acquisition
must not remove a token without verified recovery. A normal confirmed return
removes its token. Do not use a PID, sensor label or GUI request as a token.

Begin maintenance atomically closes admission and returns all current tokens.
With sessions it enters draining, otherwise ready. A timeout of 100..30000 ms is
fixed at begin; retries cannot extend it. The supervisor must stop issuing curve
commands, request explicit return through each protected session, and poll even
when a worker stops responding.

For each exact token, release requires all of:

- Native ownership relinquished.
- Backend-specific firmware return verified independently of callback completion.
- Durable recovery intent successfully cleared after that verification.

These must be authenticated evidence inputs, never GUI assertions. The pinned
Nct677X adapter now checks the original native mode on return before journal
clearance, and private closed receipts carry separate return/clear flags. Plain
callback completion or a closed state alone still cannot satisfy these conditions.

Timeout (including a completion exactly at the deadline), worker disappearance,
incomplete release evidence or clock reversal blocks maintenance. Outstanding
tokens remain for diagnostics/recovery. Late success cannot reopen admission or
authorize replacement. Duplicate and unknown tokens cannot clear another session.

Ready authorizes the next maintenance stage only while the protected supervisor
retains exclusive admission/lifetime ownership. It never reopens automatically.
After replacement, a new supervisor validates recovery and starts a new gate.
Gate-local tokens are not globally unique: a future authenticated transport must
bind them to a unique supervisor lifetime and reject replies from old workers.

## Remaining production integration

The Rust `CoolingHost` now owns the actual runtime tasks, reserves admission
before native acquisition and implements stop/drain with the private verified
return receipt. Tests exercise successful drain, missing evidence and stuck
acquisition. It permits only one channel until the durable journal supports more.
This is executable integration logic, but still has no productive SCM/installer
caller. See `COOLING_RUNTIME.md` for the exact remaining boundary.

The existing C# service stop path now closes diagnostic admission and waits for
bounded in-flight native work before forced termination. This gives the fixed100
diagnostic time to verify return; it is not yet an active-curve maintenance
handshake. Forced termination cannot assert return and leaves recovery intent.

An in-memory gate alone cannot exclude another process or survive a supervisor
crash. The service needs exclusive lifetime ownership, a maintenance handshake and
an OS-level admission boundary shared with upgrade/uninstall. The installer must
wait for authorization before stopping or replacing binaries and fail without
deleting recovery evidence when authorization fails. Rollback must retain that
boundary. Neither GUI participates in hardware shutdown decisions.

Current acceptance is synthetic Rust testing: multiple sessions, admission refusal,
exact/duplicate tokens, each missing proof, worker loss, timeout, clock reversal,
overflow and non-extendable deadlines. Actual service/installer integration remains
unchecked in `COOLING_IMPLEMENTATION_PLAN.md`.

The feature-gated `freergb-cooling-guard-fixture --supervised` now combines the
Rust telemetry/lease and maintenance policies in a real child process. A bounded
input thread may block on a partial message while the supervisor timer continues.
Command heartbeats and initial samples are synthetic, and test messages create
synthetic measurements at receipt time; this is not a production sampler protocol.
Release proof is likewise test data, never evidence from real hardware. Output
always reports `simulated=true` and `physical_write_verified=false`.

Run `cargo test -p freergb-daemon --features cooling-fixture --test cooling_guard_process --locked`.
The fixture is excluded from ordinary builds/installer packaging. It does not
install a service, open a sensor backend or write hardware. Production supervisor
authentication, process ownership, native I/O and installer handshake remain open.
