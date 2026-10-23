# Installation and first-run wizard

Goal: Installation -> automatic GUI start -> first-run wizard -> safety
confirmation -> device detection -> extension downloads -> fan tests -> GUI.

## Decisions (owner, 2026-10-23)

| Topic | Decision |
|---|---|
| Rights | Setup always runs elevated once (UAC at start) and installs per machine to Program Files: app, LibreHardwareMonitor runtime, .NET 4.8 if missing, PawnIO if missing, protected mainboard service. No component checkboxes. |
| Sources | LHM runtime and notices are bundled in the installer (hash-checked at build time, MPL-2.0). PawnIO has no stated redistribution permission, so the signed official setup is downloaded during installation (fixed SHA-256 + Authenticode signer) and started automatically; its own license dialog may appear once. |
| Fan test | Identification first: one header at a time is held at 100% for a few seconds; the user says which fan reacted, names the header and marks pumps. The guided channel test (100/70/50%) runs only for confirmed normal fans. Pumps and unconfirmed headers are never reduced. |
| Language | English (like the rest of the modern GUI). |
| Liability text | Names "the freeRGB developers"; limitation only to the extent permitted by law, no blanket exclusion. Legal review recommended before release. |
| Extensions | The existing signed hardware-module catalog (`HARDWARE_MODULES.md`). |
| Safety gate | Without the safety acknowledgement the daemon refuses every fan write (sessions, tests, identification, background, unlocks, profiles). RGB lighting is unaffected. |

## Flow

1. **Setup** (`packaging/windows/freergb.iss`): elevated, no task/component
   pages except shortcuts. A progress page shows each step (app files, .NET,
   PawnIO download/verify/install, LHM runtime, protected service) with
   understandable error text and a retry. Already present components are
   detected and skipped. At the end the GUI starts automatically as the
   original (non-elevated) user.
2. **Wizard** (modern GUI, `apps/desktop/src/features/onboarding`): opens on
   first start (daemon `onboarding.get` reports `completed:false`), can be
   reopened from Settings. Steps: Welcome -> Safety (red) -> Devices &
   extensions (existing module setup) -> Motherboard & fans -> Summary.
3. **Daemon**: `safety.get` / `safety.acknowledge` (versioned text, stored in
   the per-user config) gates all fan control; `onboarding.get` /
   `onboarding.complete` / `onboarding.reset` track the wizard; session mode
   `identify` holds one header at 100% for a bounded time, then returns it to
   firmware.

## Safety

RGB lighting and fan control stay separated: the wizard's lighting part only
uses the module/HID path; fan control only the protected mainboard service.
Identification never lowers a duty. The channel test never goes below 50%
and excludes pumps. All existing guards (journal, watchdog, firmware return,
critical temperature) apply. Control keeps running with the GUI closed
(daemon + protected service).
