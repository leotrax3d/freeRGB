# Testing

## Automated
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
2026-10-18: 94 tests passing on Windows 11 (Rust 1.99.1). CI (`.github/workflows/ci.yml`) builds and tests on Windows and Ubuntu; Linux hardware access is untested.

Categories:
* **Unit:** core colours/layouts/canvas, effects, report descriptor parser, packet encoding.
* **Simulation:** `corsair_bragi::device::tests` (simulated K70 with mode/handle state),
  `engine::tests` (virtual devices, reconnect back-off), `api::tests`, `ipc::tests`
  (JSON-RPC over an in-memory duplex stream).
* **Hardware:** manual, below.

## Hardware tests – K70 CORE (manual)
Precondition: SignalRGB stopped (`Stop-Process -Name SignalRgbLauncher,SignalRgb`).
1. `freergb bragi probe` → pid `0x1bfd`, read mode.
2. `freergb bragi color #ff0000 --seconds 20` → fully red, then hardware mode.
3. `freergb bragi keys Esc=#ffffff "Left Ctrl=#ff0000" Fn=#ffff00 …` → only those keys lit.
4. `freergb bragi key-walk` → one key at a time; compare with printed names.
5. `freergb bragi effect rainbow --fps 60` → prints achieved FPS.

## Daemon end-to-end (manual)
1. `freergbd --virtual-device`
2. `freergb ctl devices.list` → K70 + virtual strip `Connected`.
3. `freergb ctl color.set '{"color":"#00ff00"}'`, `freergb ctl diagnostics.get`.
4. `freergb ctl daemon.shutdown` → process exits; `freergb bragi probe` shows mode 1.

Results: `DEVICES.md`.

## GUI smoke test (manual)
`freergbd --virtual-device --no-hardware`, then `freergb-gui`: window shows
"daemon connected" and the virtual strip; changing the effect via `freergb ctl` keeps
the GUI alive. 2026-10-18: passed on Windows (start + state change). Interactive
controls were not clicked by an automated test yet.

## iCUE LINK hub (manual, needs user approval – fans and pump are attached)
Precondition: SignalRGB stopped, no `freergbd` running.
1. `freergb link-probe --raw` – read-only, mode must stay 1.
2. `freergb link-probe --software-window` – children, RPM, temperatures; prints
   "hardware mode verified".
3. `freergb link-light-test --seconds 10` – fans red/green/blue, pump head white,
   LCD ring magenta; RPM watchdog; hardware mode verified.
4. Soak: `freergb link-light-test --seconds 900` – check `glitches` and RPM min/max.
5. `freergbd --enable-link-hub`, `freergb ctl color.set …`, `freergb ctl daemon.shutdown`,
   then `freergb link-probe` must show mode 1.
Results: `DEVICES.md`.

## Continuation checks — 2026-10-19
Windows (Rust/Cargo 1.99.1): 97 workspace tests pass, workspace Clippy with
`-D warnings`, formatting check and workspace release build pass.
New HID gate tests cover FIFO admission (including owner re-entry), mutual
exclusion, panic release and ticket-counter wraparound without real hardware.

Run `./scripts/smoke-daemon.ps1` after the release build, with no existing daemon.
It starts its own hidden daemon with `--no-hardware --virtual-device --no-state`,
checks the named-pipe API and color round trip, waits for a rendered frame, and
shuts down its own process. Passed on Windows on 2026-10-19.
Hardware FPS/soak measurements and Linux validation of this change remain open.

## Process-plugin milestone — 2026-10-19

Final Windows checks: **109 workspace tests pass**, workspace Clippy with
`-D warnings`, `cargo fmt --all -- --check`, `cargo build --workspace --release`,
and `git diff --check` pass. Logs are local in `target/codex-tests.log` and
`target/codex-release.log` (ignored build artifacts).

`./scripts/smoke-daemon.ps1 -WithPlugin` passed against release binaries:
real named pipe, virtual rendering, color round trip, two real plugin children,
one deliberately crashing while the healthy child keeps receiving frames,
plugin stop/start, daemon shutdown. Process inventory afterward showed no remaining
`freergbd` or `freergb-plugin-virtual` processes. SignalRGB stayed running;
tests used `--no-hardware --no-state` and no real devices.

New regressions cover manifest bounds/path resolution/unknown fields, exact RGB
transfer and reconnect, blocked child stdin, hang/crash/bad version/bad ID/bad JSON/
oversized replies, hardware-disabled allowlists, invalid saved profiles/effect layers,
and IPC requests at/above the exact 256 KiB limit.

During development, an IPC test initially waited for EOF without shutting down its
client write half; fixed the test lifecycle. The profile test's new corrupt fixture
initially changed its expected directory listing; now cleaned up within the test.
One process fault test exceeded its 5-second total wall-time check on a workspace
run; total includes OS startup as well as timed I/O. It now allows 15 seconds for
the total and separately asserts the timeout error; the actual 500-ms test protocol
deadline is unchanged. Subsequent full checks passed. No precise cause for the
transient startup/run delay was established.

The Windows release smoke is added to CI; CI has not been pushed/run for this
change. No local Linux or real-hardware run, and no automated GUI click test.

## Cooling/macros/distribution — 2026-10-19

Windows: 116 workspace tests pass; Clippy with denied warnings, formatting,
debug/release builds and the expanded release IPC smoke pass. New coverage includes
curve interpolation/hysteresis/stale-input policy, macro scheduling/cancel/input gate,
definition persistence, semver/asset selection and tamper detection. LINK lifecycle
simulator also verifies cached RPM publication. No test sends keyboard input or new
cooling commands to real devices.

Installer compilation with official Inno Setup 6.7.3 passed; log
`target/installer-build.log`, executable/checksum under `target/dist/`. No live
install/upgrade/uninstall or real updater version transition has been performed.
New GUI panels compile; interactive clicks are not automatically tested.

GitHub Package workflow dispatched; artifact runner last observed queued.
Older CI Windows passed, Ubuntu failed missing PipeWire libraries; CI dependencies
were corrected, fresh Linux confirmation pending. Do not carry forward an unqualified
claim that all current Linux checks are green.

## Visual macros / history / diagnostics — 2026-10-19

120 Windows workspace tests pass. Clippy with denied warnings, formatting, release
build, Inno installer compilation and the real IPC smoke (including redacted report
export) pass. Regression tests cover failed chord presses with release attempts,
chord validation/input classification, sensitive macro/custom-device-name exclusion
from reports, missing/stale history gaps, expiry and capacity during long sample runs.
Native keys are never injected by tests. GUI controls/graphs compile; automated click
and actual chord output acceptance remain untested. GitHub Package/Linux results are
pending runner availability, independent of these local checks.

## Global hotkeys and resilient client — 2026-10-20

128 Windows workspace tests pass, formatting and Clippy with denied warnings pass.
Release build, virtual-device IPC smoke (including plugin crash recovery, curves,
RGB macros and diagnostic export) and Inno installer compilation pass.
Client regressions include a silent real Windows named pipe and bounded replies.
Hotkey tests validate bindings and registration rollback through a fake backend;
queue tests preserve action order while coalescing slider changes per device.
No native hotkey registration, foreground keyboard input or GUI clicks are tested.
No real cooling writes or product install/upgrade/uninstall were performed.

GitHub Package run 39285464153 for 740b5b8 completed successfully; its downloaded
installer checksum was verified. Later Ubuntu CI exposed missing ALSA development
headers; libasound2-dev is now included, pending a fresh runner result.

## Sensor warnings — 2026-10-20
132 Windows tests pass, including low/high debounce and hysteresis, brief excursions,
acknowledgement recurrence, invalid/duplicate rules and stale/disconnected/changed
unit readings. Tests use cached JSON and simulated time, with no hardware access.
Formatting, denied-warning Clippy, release build, virtual daemon/plugin IPC smoke
and regenerated Inno installer pass. GUI click and real sensor acceptance remain
open. This is advisory monitoring only; details in MONITORING.md.

## Foreground application profiles — 2026-10-20
136 Windows workspace tests pass. Four new tests cover focus stability/rapid changes,
no repeated load, re-entry, macro deferral, missing focus, executable validation and
invalidated command rejection. No test reads the real active window or applies a
profile to hardware. Formatting, Clippy with denied warnings, release build, virtual
IPC/plugin smoke and installer compilation pass. Actual focus-switch acceptance
remains open; see APPLICATION_PROFILES.md.

## Installer hardening — 2026-10-20
137 Windows workspace tests, fmt/Clippy, release build and guarded IPC/plugin smoke
pass. The guarded shutdown regression checks current executable, a different path
and malformed relative input. Inno Setup compiles with German/English resources and
the project icon. The compiled Pascal harness verifies supported/unrelated product
names, publishers, quoted paths, MSI /I→/X, ambiguous paths and shell/script rejection;
it exits during initialization, before installation. PowerShell harness parses.
No actual product install or third-party uninstall ran on this personal machine.
The Package workflow now performs lifecycle acceptance on a fresh Windows runner;
its new result remains pending. Same-version reinstall/upgrade-path testing does
not establish cross-version updater or interactive vendor-uninstaller acceptance.

## LINK telemetry and restore ownership — 2026-10-20
143 Windows workspace tests pass, including six new regressions for strict table
validation, signed and implausible temperatures, cache-only reads/failed-read age,
RPM watchdog operation without temperatures, restoration retries/drop and foreign
session preservation. Formatting, denied-warning Clippy, release build, virtual
IPC/plugin smoke and Inno packaging pass. New temperature polling is simulator-tested,
not live-tested. No cooling writes and no independent process watchdog implemented.

Package run 39371595581 reached actual fresh installation on its runner, then failed
because the harness expected a custom Start Menu group. Logs prove installation
created the configured freeRGB group and startup entry. Harness expectation fixed;
remaining reinstall/uninstall/profile preservation results require a new run.


## Custom graphs and optional motherboard process provider — 2026-10-20

Pure graph evaluation, versioned persistence, rejection without mutation and
request-time stale handling are tested independently of egui. The provider's
snapshot validation rejects unknown protocol/status/kinds, duplicate identities,
oversized inventories/messages and out-of-range/nonfinite data. RPM and control
channels remain distinct, unpaired, unknown electrical mode and never writable.

Run `./scripts/test-mainboard-provider.ps1` on Windows for the ignored process
acceptance test. It compiles our own helper plus synthetic no-hardware fixture,
then tests actual stdin/stdout framing, missing-runtime status, valid data,
malformed/oversized replies, early exit, 10-second initial deadline and bounded
cleanup, including cancellation during an active native request. Temporary paths contain spaces and Rust's extended Windows prefix.
LHM is never loaded by this test; no driver or hardware is opened.

Real library/driver-backed enumeration across mainboards and safe cleanup still
require hardware validation. In particular stock LHM Close requests control resets;
our read-only process does not call it. Do not equate process-fixture success with
verified hardware support. The virtual API smoke checks disabled-provider status,
empty channel inventory and missing custom-source values.


Current Windows verification: 155 regular workspace tests plus the separately run
ignored provider transport acceptance test passed; fmt, denied-warning Clippy,
release binaries, virtual IPC/plugin smoke, installer model checks and local Inno
Setup compilation passed. No actual installer or driver was run on this PC.


## Optional prerequisite installation and ASUS Aura RGB

`scripts/test-mainboard-prerequisites.ps1 -VerifyDownloads` passed in PowerShell7
and Windows PowerShell5.1. It tests explicit-consent rejection, runtime/driver
presence plans, invalid manifest/hash/unsigned installer rejection, official hashes
and executable signatures, runtime DLL staging and accompanying license/source
notices. No upstream installer or driver is executed by these tests. Actual opt-in
wizard/UAC/manufacturer-install/reboot acceptance remains open.

ASUS mock transport tests cover malformed firmware tables before mode writes,
20-LED packet chunks, ninth-offset addressing, selected-header layout, allowed RGB
commands only, frame bounds, spectrum fallback and restore failure ownership.
Daemon/launcher tests cover explicit flags, exactly four 0..512 manual lengths,
demo exclusion and bounded/fixed-path installed-state consent. No live ASUS LED
change or Gen1/Gen2 validation has been performed.

The Windows smoke harness uses a unique `--ipc-instance`, required to combine
`--no-hardware --no-state`; it can run beside a Tauri preview without sending any
command to that preview's normal endpoint. Both GUIs/SDK keep the original pipe.


Local completion checks: 162 regular Windows tests (161 full workspace plus the
new isolated-endpoint regression run in the targeted suite), separate ignored
provider transport acceptance, denied-warning Clippy, release build, isolated
virtual IPC/plugin smoke, PowerShell5.1/7 prerequisite verification/staging, Inno
build and installer model checks passed. Tauri preview PID32124 remained running
on its default endpoint. No personal-machine installer, driver install or live
ASUS color test was performed.

## Read-only broker boundary candidate

`scripts/test-mainboard-broker.ps1` compiles the helper, service candidate and a
fixture using the .NET Framework compiler, without installing a service/driver or
opening hardware. Real named-pipe tests cover account-specific DACL rights (no
Everyone/anonymous/create-instance), exclusive name acquisition, server image
checks, identification-only client token, command validation, bounded/strict UTF-8
replies, silent peers, pending-I/O cancellation and reconnect/acknowledgment using
one persistent handle. Cache tests cover retained age, expired snapshots/values,
malformed envelopes, unsupported versions and future/negative/fractional clocks.
Unprotected worker startup and writable payload paths are rejected.

The fixture passes in PowerShell 7 and Windows PowerShell 5.1; existing ignored Rust
helper transport acceptance also passed and Inno compiled. These are targeted
checks for this increment, not a new full-workspace/hardware acceptance run.
CI and Package now run the broker fixture. Protected deployment and actual SCM/
native-worker lifecycle were still untested at that stage; see the subsequent
protected-deployment/SCM acceptance below.

## Protected deployment and SCM acceptance

`scripts/test-mainboard-deployment.ps1 -VerifyRuntime` passes disk-backed install/
update/rollback cases including failed new start, prior stopped/running state,
initial install failure, failed stop/swap, wrong hashes and locked verified sources;
it also extracts the pinned real runtime/notices without loading them. It passed
in PowerShell7 and Windows PowerShell5.1. Local Inno compilation/model checks,
consent gates and existing Rust helper transport regression passed.

Mainboard broker run **39469765979 succeeded** on a disposable elevated Windows
runner: actual SCM install with protected image/DLLs, a separate real standard-user
process reading through the broker, failed-start byte/state rollback, 900-to-901
version update, changed-helper rejection, foreign-owner removal rejection and owned
uninstall. This uses a compile-time synthetic worker, no driver/hardware access.
The prior run caught a real normal-user identity-query rejection; the corrected
client authenticates SYSTEM pipe ownership plus SCM service identity instead of
requiring SYSTEM process-image query rights. A second unauthorized standard-user
test and isolated synthetic-child crash/restart also passed in run 39470499850,
without restarting the service process.

Setup now wires this maintenance to explicit mainboard opt-in. Default/silent
installer lifecycle also checks no privileged service is created and both
maintenance/removal helpers are installed/removed. These new final installer
checks need their own Package result. Actual wizard/UAC/vendor installer/reboot,
native worker crash/hang, power-loss repair and real motherboard support remain
unverified. No service/driver was installed on the developer PC.

## Keyboard reactions

169 workspace tests passed for the integrated keyboard overlay. Targeted tests
also cover stale events buffered in the input pipe, future capture timestamps,
handshake/version bounds, ISO/ANSI mapping, base-effect preservation, brightness,
failed persistence and immediate runtime disable. Denied-warning workspace Clippy
and release compilation passed. Local native helper checks use `--self-test`
only and do not register a keyboard hook. Disposable Windows run 39474499240
passed actual hook startup and immediate stdin-EOF cleanup without recording
events. Complete-feature CI/Package and real typing-to-LED checks remain pending.

Package39470839127 passed the prior installer changes, including the actual
owned mainboard-service uninstall hook; it supersedes the pending Package note
above. It does not validate real driver/hardware behavior.

## Motherboard channel annotations

173 workspace tests passed after adding control/RPM/header annotations, including
duplicate assignment rejection, identity/kind conflicts, stale/offline resolution,
bounded storage failure behavior and additive RPC behavior. Workspace Clippy with
denied warnings and the complete release build passed. The release IPC smoke
checks an offline assignment remains unavailable/unverified/nonwritable and can
be cleared under `--no-hardware --no-state`. It also covers the existing keyboard,
curves, macro, virtual rendering and process-plugin paths. Inno model tests passed.
These tests exercise no real fan writes, driver installation or keyboard capture.

Updated keyboard helper lifecycle run39475875921 passed on disposable Windows,
including the final capture timestamp implementation. Full combined CI/Package
results for each commit must be checked independently; native hook cleanup is
not a substitute for real typing-to-LED or fan-header hardware validation.

## Bounded calibration and independent-process policy acceptance

Keyboard Package39475877313 and channel annotation CI39476917007 /
Package39476933684 succeeded. The latter covers both CI OSes and the complete
Windows installer/release lifecycle for the preceding increment.

The new cooling policies pass full workspace tests and targeted fan response,
late/missing acknowledgment, stale/repeated sensor acquisition, stall/temperature,
unstable/constant RPM, reversed-clock and final-restoration fault tests. Results
cannot be reported before acknowledged full-speed restoration. Denied-warning
workspace Clippy with all features, release build, release IPC smoke (including
the new stateless calibration preview) and Inno compilation/model passed locally.

Three actual-process integration tests pass on Windows with feature
`cooling-fixture`: a hung producer remains alive while the separate guard requests
simulated 100%, killing the producer causes EOF/full-speed fallback, and a 513-byte
record without newline cannot block the guard. CI/Package run these explicitly;
the synthetic-only binary is excluded from normal build/installer. This proves
process independence and decisions, not physical fan speed, a successful register
write or recovery after the guard itself/driver/OS fails. No service, driver or
keyboard hook was installed/activated on the development PC.

## Native control adapter candidate

`scripts/test-mainboard-control.ps1` passes in PowerShell7 and Windows
PowerShell5.1 against the hash-pinned official LHM0.9.6 runtime. Actual Control
objects use synthetic sensor/settings interfaces and event callbacks only; no
Computer/SuperIO is constructed or opened. The baseline reproduces the public
setter's intermediate zero. The adapter tests cover no-zero acquisition, repeated
duty reassertion, wrong object type, duplicate/unowned/foreign/lost mode rejection,
finite fan/pump bounds, selected restoration/idempotency, partial acquisition/write
failure, retained ownership after failed restoration and retry despite cached
Default mode. These checks do not prove physical register writes or safe RPM.

Production helper/service compile and broker boundary tests pass, including
rejection by the protected production factory and absence of the test-only factory
in production compilation. Deployment disk/verified-runtime/rollback tests and
Inno compile/model pass. No public setter/activation flag was introduced. Existing
Rust source/API types remain unchanged for this increment. Native recovery during
driver hangs, authenticated session transport and physical readback are still open.
See `MAINBOARD_NATIVE_CONTROL.md` for the exact trust boundary and private metadata
dependency; widening the version check alone is not an upgrade procedure.

## Private native-session candidate

Local PS7/PS5.1 acceptance passes one-use ticket/counter/shape validation, delayed
and replayed request rejection, server-clock expiry, late completed write fallback,
pump bounds, failed recovery, explicit close and real-LHM synthetic 100/40/100
integration. Real named pipes test stalled input, disconnect and an oversized
unterminated record; fallback occurs independently of the longer input wait and
does not silently release the reservation. Protected production factories reject
unprotected processes; test factories/runners are absent from production builds.

The new SCM fixture additionally requires real registered service-to-worker client
PID/config/SYSTEM-token proof. Its fresh disposable GitHub result is pending;
no SCM service/driver is installed locally. Prior native adapter Package39479651725
and quick/broker workflows39479651926/39479651985 succeeded. No test proves physical
fan speed or recovery from a blocked native driver. Scope/limits and private
protocol are documented in MAINBOARD_NATIVE_SESSION.md.
