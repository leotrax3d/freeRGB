# Mainboard SMBus driver owner decision (P3 / S23)

Status: **owner decision pending**. This change implements software interfaces;
no driver is selected, installed, loaded or redistributed. Native access remains
an explicit host integration, absent from normal daemon startup and module setup.

## Decision to record before Windows activation

Owner: project owner. Record the chosen driver/version, artifact hash, licence review,
distribution method, signing evidence and approved controller recipes here.
Recommendation: official signed PawnIO with reviewed, signed controller modules,
subject to owner approval. This recommendation does not constitute owner approval.

| Option | Licence and distribution | Signing and decision |
|---|---|---|
| Official PawnIO | Official binaries have separate distribution terms; do not equate them with the source licence. Prefer an owner-installed upstream prerequisite; bundling needs redistribution review. | Publisher describes the official edition as digitally signed. Verify the exact pinned release, publisher chain, module signatures and supported Windows security configuration. Recommended candidate, pending approval. |
| Build PawnIO from source | Driver source: GPL-2.0-or-later with a special exception for specified IOCTL combinations; Pawn modules are expressly outside that exception. Audit PawnIOLib and every loaded module separately. | A source build does not acquire the official signature. Requires owner-managed Microsoft driver signing and module signing/distribution arrangements. No unsigned/unrestricted production edition. |
| WinRing0 / generic raw port drivers | A permissive licence alone does not settle security or distribution suitability. | Rejected as a design option: broad raw kernel access conflicts with the narrow transaction boundary. No fallback implementation. |
| Custom constrained driver | Owner chooses licence, distribution and source obligations; review controller code independently. | Requires a maintained kernel implementation, applicable Microsoft signing process, release ownership and security maintenance. Not implemented here. |
| Linux i2c-dev | OS interface through `i2cdev` 0.6.2 (MIT OR Apache-2.0); no kernel driver bundled. | Administrator provisions adapter and permissions separately. No module loading, adapter scan or permission changes in this code. |

Primary sources reviewed 2026-11-08:
[PawnIO distributions](https://pawnio.eu/),
[driver source and exception](https://github.com/namazso/PawnIO/blob/master/README.md),
[upstream usage/licensing guidance](https://github.com/namazso/PawnIO.Modules/wiki/Using-PawnIO-Modules),
[i2cdev licence and API](https://docs.rs/i2cdev/0.6.2/i2cdev/),
[Microsoft driver signing](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-policy).
Exact selected artifacts and their licence texts still require owner review.

## Implemented software boundary

`bus_simulation::Bus<B>` is the shared API 2 broker core. `SimulatedBus` remains
an alias using the original register simulator; existing fixtures remain intact.
`Bus<NativeBackend>::start_module` verifies the installed module before launch and
copies its signed policy into its private session. It checks consent, PCI identity
and policy backend against the host-bound backend. Each session has a distinct
capability. Calls cannot provide adapter paths, driver names or raw port numbers.
The complete batch is checked against that module's address/register/operation/
length/write-mask rules before the first operation. Sensitive-address write
exclusions, flat-register restrictions, queue limits and deadlines remain.
Banked controllers and raw I2C are outside API 2.
API 2 accepts the new `native` hello transport label and the existing SDK's
`simulation` label for compatibility. The hello never chooses the backend;
only the verified policy and host-owned backend binding grant a session.

`NativeBackend` serializes native operations on one worker per physical adapter.
The host must retain one shared `Bus` for every canonical adapter across modules.
A native error, malformed reply or timeout faults the shared bus; no retry,
rollback write or automatic replacement occurs. Caller deadlines are bounded,
but an already-issued kernel operation may finish after timeout. The supervisor
must quarantine the physical adapter until that worker exits. A worker thread
cannot cancel a stuck kernel call; production helper isolation/lifecycle and
adapter ownership must enforce this across disconnects and restarts.

Linux `LinuxDevice::adapter(number)` binds `/dev/i2c-N` from trusted host input.
It opens only on an authorized transfer, uses non-force slave selection, SMBus
byte/block ioctls and combined I2C register-select/read (repeated START).
I2C register writes use one register-plus-data message. No scanning or fallback.
SMBus block reads require full 32-byte register permission before opening the
device, because the device controls the wire count; short native results fault
under API 2's exact-length contract. Use I2C register reads for shorter fixed
reads where supported. Native adapter errors fail closed.

Windows `WindowsDevice<D>` implements all six API 2 operations against
`DriverInterface`, tested with a memory fake. It has no PawnIO DLL bindings,
IOCTL implementation, driver discovery, service changes, loading or installation.
The eventual adapter must bind a protected controller recipe and must not expose
arbitrary Pawn scripts, IOCTLs or ports to modules.

## Protected helper integration and activation gate

This is an opt-in Rust broker core, **not a new privileged RPC endpoint**. The
protected sensor service in `MAINBOARD_BROKER.md` remains read-only and cannot
accept these bus calls today. Production activation requires separately reviewed
protected worker/IPC integration preserving its authenticated client, protected
package/path and fixed executable guarantees. Never instantiate native backends
in a module process or silently bypass the broker. Admitting native policy names
does not enable hardware. No module ports are included in this change.

Hardware acceptance is tracked only in
[07-hardware-acceptance.md](promts/fancontrol/07-hardware-acceptance.md).
Synthetic tests do not establish hardware acceptance.

## S23 verification

- `cargo fmt --all --check`: passed.
- `cargo clippy -p freergb-module-api -p freergb-modules -p freergb-daemon
  --all-targets --features freergb-daemon/bus-fixture --locked -- -D warnings`:
  passed.
- `cargo test -p freergb-module-api -p freergb-modules -p freergb-daemon
  --locked -- --test-threads=1`: passed, including integration tests and doctests;
  existing opt-in fixture tests remain ignored. Daemon library: 163 passed,
  1 ignored; module API: 35 passed; modules library: 27 passed.
- `cargo test -p freergb-daemon --features bus-fixture --test bus_process
  --locked`: 6 passed, including a signed native-policy package through the real
  child lifecycle with a fake native device. No hardware opened.
- Linux source cross-target clippy: passed for `x86_64-unknown-linux-gnu` in an
  isolated scratch crate importing the exact `bus_native/linux.rs`, the actual
  module API crate and pinned i2cdev 0.6.2. This checks the Linux binding without
  requiring desktop Linux system libraries or executing native I/O; it does not
  establish a complete Linux daemon build or runtime acceptance.

The PowerShell gate could not start because local script execution is disabled;
the allowed Cargo equivalents above were used without changing machine policy.
An initial parallel run exposed an existing cooling startup/sample-age timing
failure; the complete serial run passed it. A new quarantine test's incorrect
expectation for the already-revoked session was corrected: that session receives
Denied and other sessions receive Faulted. All final source checks passed.
