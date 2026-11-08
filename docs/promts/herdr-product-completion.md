# Big prompt for herdr — product completion outside fan control and device ports

This is one large task for a team of agents. A **coordinator** agent reads this
file, assigns the workstreams below to worker agents (one workstream per agent,
run in parallel), reviews their PRs and keeps the shared files consistent.

## Context
freeRGB (repo `leotrax3d/freeRGB`) is a Rust RGB + fan-control suite with a
daemon (`freergbd`), a Tauri/React desktop app (`apps/desktop`), a Classic
Win32/egui GUI (`crates/freergb-gui`), a tray, an installer and signed
downloadable hardware modules. Read first: `docs/HANDOFF.md`, `docs/STATUS.md`,
`docs/ARCHITECTURE.md`, `docs/LOCAL_API.md`, `docs/DECISIONS.md`,
`docs/SECURITY.md`, `docs/TESTING.md`.

## Other people work in parallel — never touch their areas
| Area | Owner | Paths |
|---|---|---|
| Fan control | Claude session (prompts 08–10) | `crates/freergb-control` (cooling, curves, channels, calibration, fan_support), `crates/freergb-daemon/src/cooling_*`, `fan_support.rs`, `hwmon.rs`, `mainboard.rs`, `background.rs`, `crates/freergb-gpu-*`, `providers/mainboard/`, `apps/desktop/src/features/cooling/`, `docs/COOLING*`, `docs/FAN*`, `docs/FANCONTROL_PARITY.md`, `docs/promts/fancontrol/` |
| CI and installer build | Claude session (prompt 10) | `.github/workflows/`, `packaging/windows/`, `scripts/build-*.ps1` |
| OpenRGB device ports | a friend | `modules/`, `scripts/openrgb/`, `docs/OPENRGB_*` |

If a workstream truly needs a change there, stop and report it instead.

## Rules for every worker
- One branch per workstream from current `main` (`<ws-id>/<topic>`), own git
  worktree, never switch a shared checkout. One PR to `main`; delete the branch
  after merge. Search for existing code first and extend it — no parallel
  implementations.
- **No UAC/elevation, no installer runs, no service changes, no hardware
  writes, never use any password.** The owner is away. Everything is verified with
  mocks, fixtures, simulators and CI. Collect real-hardware/owner steps in the
  workstream's doc under "Owner acceptance steps".
- Repo language English. Public demo/mock data must not contain real hardware
  brand/model names (checked by `scripts/build-website.mjs`).
- Done = `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`, and for `apps/desktop` lint/typecheck/tests,
  all green locally and in CI.
- **Shared files** (`docs/STATUS.md`, `docs/HANDOFF.md`, `docs/LOCAL_API.md`,
  `README.md`, `Cargo.lock`, `apps/desktop/src/lib/mock/transport.ts`): change
  only your own lines/sections, rebase on `main` right before opening the PR.
  Only check a STATUS box with evidence; implementation and acceptance are
  separate milestones. If a box changes, regenerate the README progress block
  (`scripts/update-readme-progress.py`; port its logic if Python is missing).
- Work time-efficiently: long builds/tests in the background, keep working.

## Workstreams (each is independent; file ownership in brackets)

### WS1 — Macro recording and per-device macro keys  (STATUS: Automation)
[`crates/freergb-daemon` macro/hotkey code, `crates/freergb-gui` macro page,
`apps/desktop/src/features/macros/`, `docs/MACROS.md`]
Record key down/up, text, optional mouse buttons with delays into the existing
step format (coalesce, cap length); API `macros.record.start|stop|status`; Tauri
and Classic record buttons; visible recording indicator (GUI + tray); privacy
stops (focus loss, timeout, secure desktop/password field where detectable);
nothing stored until saved. Assign macros to device keys exposed by protocols
(start with the K70 CORE mapping used for reactive lighting) besides global
shortcuts; reject duplicates/reserved keys. Tests for recorder state machine,
coalescing, privacy stops, API validation, GUI with mocks.

### WS2 — Effect plugins and install/hot reload  (STATUS: Hardware and extensibility)
[`crates/freergb-effect-host`, `crates/freergb-effects`, `crates/freergb-plugins`
(effect side only), `effects/`, `apps/desktop/src/features/effects*` or
marketplace views, `docs/PLUGIN_SYSTEM.md`, `docs/EFFECT_*`]
Finish effect plugins: versioned contract, sandboxed host process, install from
the marketplace/collection with signature/hash checks, enable/disable, hot
reload on file change without restarting the daemon, crash isolation and fallback
to the previous effect. Tests: malformed/slow/crashing plugin, reload during
render, uninstall while active.

### WS3 — Linux parity and distribution packages  (STATUS: Distribution and platforms)
[`crates/freergb-audio`, `crates/freergb-screen`, Linux input helper code,
`packaging/linux/`, `docs/INSTALL.md` Linux section]
Audio-reactive (PipeWire/PulseAudio monitor), screen capture (PipeWire portal
on Wayland, X11 fallback), keyboard input for reactive effects (evdev with
documented permissions) — behind the same APIs as Windows. Packages: `.deb`,
`.rpm`, AppImage and/or Flatpak with udev rules and the systemd user service.
Add Linux build/package steps as scripts only (workflow wiring is done later by
the CI owner — write the exact step snippet into the PR description).

### WS4 — Updates and release tooling  (STATUS: Distribution and platforms)
[`crates/freergb-update`, `crates/freergb-module-tool`, `docs/UPDATES.md`,
`docs/HARDWARE_MODULES.md` release section]
Cross-version update and failure recovery: simulate download/verify/stage/swap
with interrupted downloads, bad signatures, rollback after a failed start, disk
full; prove no partial install can start. Module release tooling: one command
that builds, packs, signs (key path given by the owner, never committed) and
produces the catalog + release notes for the `modules` release; dry-run mode
for CI. No real publishing — write the owner steps.

### WS5 — Desktop app usability, accessibility and German UI  (STATUS: Lighting and desktop)
[`apps/desktop/src` except `features/cooling/` and `features/macros/`;
`docs/GUI_MODERNIZATION.md`, `docs/screenshots/`]
i18n framework with English + German for all non-cooling, non-macro views
(cooling/macros add their strings later through the same framework — document
how); keyboard navigation and screen-reader labels; consistent empty/error
states; foreground-application profile switching UI polish; updated
screenshots from the mock transport.

### WS6 — Motherboard RGB configuration and ASUS fallback  (STATUS: Hardware and extensibility)
[`crates/freergb-protocols` (asus_aura, gigabyte, asrock RGB), daemon
`mainboard_rgb` code, `apps/desktop/src/features/*mainboard*rgb*`,
`crates/freergb-gui/src/mainboard_rgb.rs`, `docs/MAINBOARD_RGB*.md`]
ASUS header colour order/fallback per header, LED count detection where the
protocol allows, configuration UI in both GUIs, per-header test pattern (only
as an owner step), import/export with profiles. Byte-exact protocol tests.

### WS7 — User documentation and website  (no STATUS box)
[`website/`, `docs/WEBSITE_CONTENT.md`, `docs/INSTALL.md` Windows section,
new `docs/USER_GUIDE.md`]
A user guide (install, first run, lighting, profiles, macros, fan control
basics with safety notes, troubleshooting incl. "motherboard reader is
starting" and pipe-busy recovery), website pages generated from the existing
compatibility/coverage JSON (`docs/openrgb-compatibility.json`,
`docs/fan-coverage.json`) — read-only consumers, never edit those files.

## Coordinator duties
1. Start one worker per workstream (WS1–WS7), give each this file plus its WS id.
2. Watch for overlap; if two workers need the same file, decide the owner.
3. Review each PR against its workstream and the rules; merge only when CI is
   green. Merge order: WS4, WS2, WS6, WS1, WS3, WS5, WS7 (later ones rebase).
4. At the end, write a short summary into `docs/HANDOFF.md` (repo state, what is
   simulated only, owner acceptance steps per workstream).
