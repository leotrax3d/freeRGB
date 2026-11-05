# Boot-time cooling service, profiles and CLI

Added 2026-11-04 (FanControl parity prompt 04). Everything here is covered by
simulated tests only; hardware acceptance is listed for prompt 07 at the end.

## Decision: one evaluator, in the Rust daemon

Curve evaluation for every controller type (graph, manual, linear, composite,
RPM targets, output shaping) lives in `freergb-control` and runs in the Rust
daemon. The protected C# service keeps only the native boundary (LHM, journal,
worker leases, firmware return). See DECISIONS.md #15.

* **Login mode** (default, previous behaviour): `freergbd` starts with the user
  session (autostart/tray) and resumes remembered assignments.
* **Service mode**: `freergbd --cooling-service` is started by the OS service
  manager at boot, headless (no GUI or login). Once the channel inventory is
  known (at most 60 s) it applies the configured **boot profile** through the
  normal policy, or resumes the remembered assignments when no boot profile is
  set. The boot outcome is reported by `cooling.service.get`.
* Selection: `cooling.service.set {mode, boot_profile}` or
  `freergb cooling service --mode service --boot-profile Quiet`. The setting is
  per-user config `cooling-service.json`; the `--cooling-service` flag is what
  actually makes a process the boot-time service.

### Windows boundary (open)

Each Windows channel start still needs the protected per-session authorization
(UAC) of `native_bridge::authorize`. Without a logged-in user that prompt cannot
be answered, so a Windows boot-time `freergbd --cooling-service` flags each
channel as needing attention instead of controlling it. Until the protected
service can authorize the channels of its already-consented background
configuration for a service-launched daemon (a C# change to the protected
boundary, deliberately not made without owner review and hardware), Windows
boot-time control keeps using the existing protected `BackgroundCooling.cs`
(simple curves with step limits, shared vectors). That mirror is the only
remaining duplicate of curve logic; it is removed once the hand-over above is
accepted. On Linux the systemd unit runs the daemon at boot; native sessions are
Windows-only, so Linux boot control currently covers kernel hwmon assignments.

## Fault supervision

`freergb_control::supervisor` is a pure, independently clocked policy:

| Component | Detection | Default |
|-----------|-----------|---------|
| Service | engine loop heartbeat stops (hang/deadlock) | 3 s |
| Boot | boot not decided in time | 90 s |
| Driver | native request outstanding too long | 1 s |
| Clock | time moved backwards | immediate |

Faults latch. In service mode the `cooling-watchdog` thread (not the engine
loop) runs the supervisor every 250 ms; on a fault it requests every channel's
return (100 %, then verified firmware mode) and exits with code 3 so the service
manager restarts the daemon. If the engine lock itself is hung, process exit
closes every native transport and each worker's lease returns its channel. The
existing per-session guards (150 ms pending-receipt deadline, telemetry lease,
critical temperature) remain the first line.

Maintenance (`MaintenanceKind::{Upgrade, Uninstall, Rollback}`) reuses the
`MaintenanceGate` drain: admission closes, every session must report native
release, verified firmware return and journal clear before the stage
(`ReplaceAndRestart`, `RemoveService`, `RestorePreviousAndRestart`) is
authorized. Incomplete evidence, a lost worker, timeout or a supervisor fault
during the drain block maintenance. The production installer path keeps using
the protected SCM drain described in COOLING_MAINTENANCE.md.

## Cooling profiles

A profile (schema version 2) is a complete configuration:

* `channels` — background (boot) curves (version-1 profiles contain only this),
* `assignments` — live channels with controller, curve, shaping and limits,
* `curve_set` — composite curves, `sensors` — custom sensor graph.

Electrical channel bindings (fan/pump role) are hardware facts and never part of
a profile, so importing a profile can never turn a pump into a fan.

Activation validates every channel under the current policy before anything
changes, then saves sensors/curve set, stops live channels that are not in the
profile (return to firmware), keeps unchanged channels running, restarts changed
ones once their previous job returned, and hands background curves to the
protected service only when they differ (one UAC). Switching: manual (API, CLI,
both GUIs), tray menu, global shortcut (Classic GUI, Ctrl+Alt/Shift+F6–F10) and
application rules (Classic GUI foreground rules, now with an optional cooling
profile per executable).

### Import/export

`cooling.profiles.export` returns
`{schema:"freergb.cooling-profile", version:2, name, source:{board, devices}, profile}`.
`cooling.profiles.import {document, name?, remap?}` checks the schema, maps
device prefixes (`/lpc/it8689e/0` -> `/lpc/nct6701d/0`; automatic when both
sides have exactly one device), and reports ids that are not present here. On a
different board or after any remap every `confirmed_fan` is cleared: the
physical confirmation must be repeated on the new hardware.

## Tests

* `freergb-control` `supervisor::tests` — boot deadline, service hang, hung
  driver, clock reversal, upgrade/uninstall/rollback drains, blocked drains.
* `freergb-daemon` `cooling_service::tests` — boot action, config, watchdog.
* `cooling_runtime::tests::simulated_boot_runs_the_profile_controllers_headless`
  — boot profile -> curve and composite controllers against simulated workers,
  every channel closes on stop.
* `cooling_profiles::tests` — validation, v1 compatibility, export/import,
  remapping, confirmation clearing.
* GUI rule/shortcut validation tests.

## Owner hardware steps (prompt 07)

1. **Login mode baseline:** save the current config as profile "Quiet"
   (`freergb cooling save Quiet`), reboot, log in: assignments resume.
2. **Service mode (Windows):** `freergb cooling service --mode service
   --boot-profile Quiet`; register `freergbd --cooling-service` at boot; reboot
   without logging in. Expected today: channels flagged "needs attention"
   (authorization boundary above) while `BackgroundCooling` keeps the background
   curves running; record `cooling.service.get` after login.
3. **Profile switching:** `freergb cooling apply Performance` / tray / shortcut /
   rule for a test executable; CHA_FAN2 follows; unchanged channels keep running.
4. **Import on the same board:** export, delete, import: confirmations kept.
5. **Watchdog:** with `--cooling-service`, suspend the daemon process (Resource
   Monitor) > 3 s: after resume nothing writes; workers returned the channel by
   lease; restart via service manager.
6. **Upgrade / rollback / uninstall** with a background curve active on
   CHA_FAN2: Setup drains (firmware mode verified), replaces/restores/removes,
   service restarts (not after uninstall); journal clear, fan in firmware mode.
