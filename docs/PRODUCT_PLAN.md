# Product completion plan

> **Historical plan.** Current, deduplicated status: [STATUS.md](STATUS.md); current entry point: [HANDOFF.md](HANDOFF.md).

Current mainboard cooling execution checklist: `COOLING_IMPLEMENTATION_PLAN.md`.

The RGB prototype is not a complete SignalRGB replacement. This plan supersedes
any earlier implication that the broad feature roadmap was finished.

The clarified FanControl requirement is broad motherboard-connected fan support
for users, alongside separate mainboard RGB/ARGB support. External drivers must
remain optional; prefer existing OS interfaces. The V284 reference backlog and
remaining acceptance are in `FANCONTROL_PARITY.md` and `MAINBOARD_SUPPORT.md`.

| Area | Implemented | Remaining acceptance/work |
|---|---|---|
| Lighting | Core, effects, layers/editor, profiles, session-opt-in Windows foreground application profile rules, device workers, K70, experimental LINK RGB and explicit experimental ASUS motherboard RGB/ARGB USB | More devices; long hardware soaks; richer layouts; native foreground profile acceptance |
| Cooling | Pure curve model/editor, interpolation, hysteresis, emergency/stale sensor policy, named storage/API, daemon custom Celsius graphs, optional isolated read-only motherboard provider, strict cached LINK RPM/temperature telemetry and owned-session restore tests | Restricted elevated motherboard broker, live acceptance, confirmed fan/pump/channel/source mapping, validated duty writes, independent process watchdog; see COOLING_VALIDATION.md |
| Macros | Visual step editor, advanced JSON, delay/repeat, RGB/effects/brightness, run/cancel; opt-in Windows text/key/media/chord output and global shortcuts | Native input/hotkey acceptance, recording, device key mapping |
| Monitoring | Daemon diagnostics, cached LINK RPM/temperatures, bounded histories with stale gaps, redacted offline report export, persistent local sensor warning rules with debounce/hysteresis and acknowledgement | CPU/GPU temperatures, confirmed coolant/channel identity; live temperature and GUI acceptance |
| Windows installer | Inno Setup recipe, project icon, independent shortcut/autostart choices (startup defaults on), optional manufacturer uninstallers, guarded daemon shutdown, CI lifecycle harness, unchecked verified mainboard-prerequisite installation; local compile and GitHub lifecycle runs 39374039676/39447319152 verified | Cross-version upgrade, interactive vendor uninstall acceptance; signing |
| Updates | Fixed GitHub Release source, stable-version selection, authenticated private access via environment, SHA-256 checks, stage/install UI, opt-in periodic checks/downloads | First published release; live update/rollback test; signing; Linux packages |
| GitHub | Private origin, CI, package workflow and draft releases for version tags | Run new pipeline, artifact acceptance, branch/release protection and signing policy |
| Linux | Shared core/IPC; existing CI configuration | New CI result, real hardware, capture/input, packaging |

## Order of remaining work

1. Validate the new build and installer artifact; exercise GUI/API macros/curves.
2. Confirm LINK channel identities and add identified temperatures without cooling
   writes; never invent CPU/GPU readings. Existing watchdog RPM cache is exposed.
3. Confirm fan/pump mapping and duty command behavior on authorized hardware.
   Model/pump constraints and independent failsafe must precede enabling a real curve.
4. Add macro triggers/recording and expand device support from evidence.
5. Run full installer upgrade/uninstall and authenticated release update tests.
6. Linux hardware/capture/input and platform packages.

No firmware, driver installation or system-wide service changes are part of these
ordinary implementation steps. Native input is only enabled by an explicit daemon
flag and a user-triggered macro, never by a test. The installer is built, not run on
the user's system automatically. Publishing a release/changing repository visibility
is separate from preparing the pipeline/artifacts.
