# Cooling acceptance record — 2026-10-22

This record separates the user's existing hardware results from verification of
the current source. No broad board compatibility or FanControl parity is implied.

## Existing hardware evidence

ASUS PRIME X870-P WIFI, Nuvoton NCT6701D, normal fan on CHA_FAN2:
`/lpc/nct6701d/0/control/2`, RPM `/lpc/nct6701d/0/fan/2`.
The other motherboard tachometer belongs to the Corsair iCUE LINK AIO and must
not be reduced as a normal fan. SuperIO temperature labels are not verified CPU
or coolant identities.

The protected Rust → service → native-worker path executed an 80% curve and
returned to firmware with checked registers and journal clearance. Calibration
executed 100/80/60/40%; its 50% recommendation intentionally has
`hardware_verified=false`. These observations were supplied in the task handoff;
they were not repeated during this source integration.

An active 80% session was drained before a same-version protected service payload
replacement. This proves neither release-to-release upgrade nor rollback nor
uninstallation. The bench-specific script is
`scripts/test-mainboard-active-upgrade.ps1`; it requires explicit probe consent.

## Current source integration

The additive session API and Classic dialog call the daemon-owned Rust runtime.
Normal setup does not grant control authorization. Each start requires explicit
fan/RPM confirmation and Windows UAC. Native telemetry, register validation,
watchdog and recovery journal remain authoritative. `available=true` only means
the daemon worker was enabled; it is not a hardware or installed-service verdict.

The separate `feature/tauri-gui` worktree was inspected without changing its files.
Its transport invokes the same local daemon API through a serialized Rust queue;
shutdown methods are excluded. Its cooling view currently offers curves and
source preview, without a live-session dialog. No Tauri/Node dependencies or
`apps/desktop` files were added to main.

## Release and hardware gates

- A live session through the new public API/Classic dialog still needs interactive
  UAC acceptance and verified firmware return on the exact confirmed fan.
- Sustained operation, temperature/RPM fault injection, reboot, native/driver hang,
  service/OS crash and power loss require controlled hardware acceptance.
- Real version-to-version upgrade, rollback, rejected/timeout maintenance, and
  actual uninstall must be accepted on a disposable release test installation.
- Wider chips/boards and motherboard RGB/ARGB require separate hardware evidence.
- The 55% overall progress value remains a rough engineering estimate, not a
  measured result or compatibility percentage.

Setup now prepares protected maintenance before replacing application files.
The service closes admission, drains the worker, verifies return and clears the
journal; the installer then checks the journal under the native maintenance mutex.
Protected payload replacement/removal retains that mutex throughout its transaction.
Refusal or missing acknowledgement stops maintenance without deleting recovery
evidence. Cancellation after preparation leaves admission closed until a service
restart; never restart around an unresolved recovery journal. Old services without
the handshake can pass only if their existing native journal/mutex gate is clear.

## Verification of this integration

Local checks on 2026-10-22:

- `cargo check -p freergb-daemon -p freergb-gui --all-targets`: passed;
  the initial unused import warning was fixed.
- `cargo clippy -p freergb-daemon -p freergb-gui --all-targets --locked -- -D warnings`: passed.
- Daemon library: 51 passed, 1 pre-existing provider fixture ignored.
- Classic GUI: 14 passed; launcher: 2 passed. Debug daemon, GUI, launcher and CLI built.
- Native control safety/session/journal tests and broker boundary tests: passed
  with synthetic ports/test storage; no hardware or service installation.
- Deployment transactions: passed, including synthetic failed-start rollback;
  no real SCM/active hardware rollback acceptance.
- Rust to C# interoperability: all 6 tests passed over real current-user pipes
  with synthetic native measurements, including calibration and sampler loss.
- `scripts/test-cooling-session-api.ps1`: passed against a real isolated daemon
  named pipe in `--no-hardware --no-state` mode. Disabled start/stop, missing and
  unknown fields and owned shutdown were checked; no UAC or hardware writes.
- Tauri branch: frontend build passed, 56 Vitest tests passed, 5 Rust tests passed.
  Tracked files were unchanged by inspection/build/testing.
- Inno Setup 6.7.3 installed through winget (verified package hash). Installer
  compiled using current debug binaries into `target/installer-validation/`;
  this is a validation artifact, not an optimized/signed release. Installer
  command/matching model tests passed. Setup itself was not installed or uninstalled.

The fresh isolated API smoke check is included in Windows CI after the release
build. CI status for the eventual commit must be checked separately; local Windows
results do not establish Linux success or disposable-runner SCM acceptance.
