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
