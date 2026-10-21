# Mainboard cooling implementation checklist

Updated 2026-10-21. This is the execution plan for the current cooling work;
`PRODUCT_PLAN.md` remains the wider product backlog. A checked test is not hardware
acceptance. Live mainboard control remains disabled.

## Architecture

Rust owns calibration, curves, telemetry policy, supervision and API orchestration.
C# is the LHM/native adapter. Existing private C# session/journal protection stays
in place until replaced by an equally tested Rust boundary; do not add more product
policy there. Both GUIs consume the same additive daemon API.

## 1. Recovery and evidence

- [x] Flush channel intent before acquisition; block new acquisition after crash.
- [x] Exercise killed worker, corrupt journal and failed restoration synthetically.
- [ ] Establish exclusive supervisor/worker lifetime across restart and maintenance.
- [ ] Define backend-specific recovery with original register/baseline evidence.
      A new LHM object's SetDefault is not proof of original firmware restoration.
- [x] Pinned Nct677X evidence adapter: persist original mode/command metadata,
      verify native command registers and restored mode; fixed100 verified on
      actual NCT6701D/CHA_FAN2, with separate measured RPM response.
- [x] Separate requested duty, callback completion, register readback and RPM
      response; never report callback completion as physical write verification.
- [x] Protected original baseline recovery after an exact owned worker crash:
      real fixed100 test passed under the same service lifetime. Other service/boot
      and driver-hang cases remain open; no old low duty is replayed.
- [ ] Test recovery failures without clearing unresolved intent or replaying duty.

## 2. Independent supervision

- [x] Pure Rust command lease policy and killed/hung producer process tests.
- [x] Private native cached telemetry interlock, synthetic tests only.
- [x] Rust telemetry policy with distinct source identities/acquisition timestamps.
- [x] Pure Rust composition of telemetry and command lease; tested mutual failure
      isolation. This is not yet the independent supervisor executable.
- [x] Feature-gated Rust process fixture: blocked partial input, stale telemetry,
      critical heat and fan stall with healthy synthetic command heartbeats;
      maintenance success, missing evidence, timeout and input loss.
- [ ] Compose telemetry, command lease and curves in a Rust supervisor process.
- [ ] Connect bounded trusted sampler updates; reject relabelled cached values.
- [ ] Exercise stalled sampler, critical heat, missing RPM, disconnect and native
      callback hangs through process boundaries. A blocked driver needs a separately
      validated recovery mechanism; a thread watchdog is insufficient.

## 3. Protected service integration

- [x] Protected service/client authentication and private expiring session tickets.
- [x] Rust async peer for the existing private session protocol: bounded I/O,
      receipt identity/counter/duty validation, cancellation drops connection.
      Transport authentication/lifetime binding must precede construction;
      no production listener or caller yet.
- [x] Rust client to actual C# session/transport over a real current-user Windows
      pipe: set/close, disconnect and connected-but-silent lease expiry; synthetic
      native sink only. Explicit script and Windows CI acceptance added.
- [x] Pure calibration state machine and synthetic API preview.
- [x] Rust native-session runtime executes curves, full calibration, bounded I/O,
      independent sampler/timer checks and explicit return. Six cross-language
      tests pass locally; see `COOLING_RUNTIME.md` for acceptance limits.
- [x] Owned Rust runtime host reserves admission before acquisition, limits to
      one native channel and drains with verified-return evidence. Healthy-command
      tests independently reject stale temperature and sustained stalled RPM.
      Production service transport/sampler and installer callers remain missing.
- [ ] Run calibration and curves through the Rust supervisor/private native client.
- [ ] Support multiple channels with backward-compatible journal reservations.
- [ ] Retain explicit user channel/source selection and conservative pump limits.

## 4. Installer and updates

- [x] Pure Rust maintenance gate: close admission, drain exact sessions, require
      release evidence, block on timeout/crash. See `COOLING_MAINTENANCE.md`.
- [x] Existing elevated maintenance shares the native reservation mutex through
      stop/swap/remove and refuses pending/invalid journal state. Local concurrent
      exclusion tested; disposable SCM acceptance passed for `983a67a`, run
      `39559255035` (unchanged service PID, executable hash and recovery bytes).
- [ ] Connect the Rust admission/release handshake to the production supervisor.
- [ ] Request return, await confirmed release, then stop/swap/remove binaries.
- [ ] Unresolved or failed release blocks maintenance and preserves recovery files.
- [ ] Test active-session upgrade/uninstall, timeout and rollback in disposable CI.
- [ ] Keep optional prerequisites/control activation opt-in; no local installation
      or hardware writes as a side effect of development tests.

## 5. Hardware acceptance and exposure

- [x] User authorized protected-service installation for hardware inspection;
      opt-in bootstrap installed the service, reused existing PawnIO, and actual
      read-only inventory succeeded. Later fixed100 diagnostics are listed below.
- [x] User attached a normal fan to CHA_FAN2; new RPM source around1240 observed.
      Selected control/2 fixed100 test measured about1324 ->1709RPM; original
      firmware mode was verified and later RPM returned around1300.
- [x] Worker-crash recovery on that fixed100 diagnostic: unchanged supervisor,
      exact owned worker termination/restart, native baseline return and empty journal.
- [ ] Verify selected fan/pump, register effects, RPM response and firmware return.
- [ ] Test command loss, sensor loss, process/service crash and restart recovery.
- [ ] Only then add documented live daemon API and both GUI integrations.
- [ ] Keep API additive and configurations versioned; document all changes.

Next implementation: supervisor process integration and maintenance admission. Do not
skip recovery, maintenance or hardware acceptance to expose a live toggle early.
