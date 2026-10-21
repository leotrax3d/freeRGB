# Cooling maintenance admission policy

`freergb-control::maintenance` is a pure Rust state machine for the protected
supervisor. It is not yet called by the installer or a production control service.
Live control remains disabled. No API or configuration format changes.

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

These are evidence inputs, not implemented hardware probes. In particular, the
existing LHM callback/session receipt does **not** currently satisfy the second
condition; no production caller may invent that proof. The existing candidate
journal ordering must also be strengthened to retain intent until verification.

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
