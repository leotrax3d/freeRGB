# Integration of main and feature/tauri-gui

Integration branch: `codex/integrate-tauri-main`. Following user approval on
October 22, 2026, merged locally into `main` by fast-forward (`b7e82ff`).
The outstanding acceptance checks below remain prerequisites for release approval.

## Decisions

- [x] Examine both branches and the existing IPC/daemon architecture.
- [x] Make Tauri the default interface; keep Classic as an optional fallback (user approval).
- [x] Windows autostart starts only the daemon (user approval).
- [x] Create an integration branch based on main.
- [x] Incorporate Tauri sources and the design specification; retain current hardware/safety implementations from main.

## Implementation

- [x] Launcher: Tauri by default, explicit Classic mode and operation without a window.
- [x] Tauri daemon startup uses the launcher's installed provider configuration.
- [x] Integrate the Windows installer, autostart and package build.
- [x] Connect motherboard sensors and provider states to monitoring.
- [x] Connect protected cooling sessions with explicit assignment, status and stop.
- [x] Check coverage of channel assignments and calibration functions: editable annotations, actual calibration through protected sessions; existing policy simulation remains a backend/Classic tool.
- [x] Configure keyboard reactions using real daemon state.
- [x] Check Classic functions (hotkeys, application profiles, updates) for migration/fallback: initially remain in Classic, launched directly from Tauri. Hotkeys and application profiles require Classic to be running.
- [x] Clearly display connection failures, stale values and missing API functions.
- [x] Restrict mock operation to an explicitly labelled browser preview; replace the hotkey design editor in desktop mode with access to the real Classic application.

## Quality assurance

- [x] Frontend: tests, type checking and production build (56 tests passed).
- [x] Tauri: Rust tests and Windows build (8 tests including fixture; Clippy without warnings).
- [x] Rust backend and launcher: relevant tests and build.
- [x] Hardware-free IPC check: existing daemon and cooling API smoke tests passed.
- [x] Check startup and closing of the real desktop window: started, later closed with Alt+F4; daemon remained reachable through the API and active at around 30 fps afterwards.
- [x] Add installer checks and CI for Tauri (local installer compilation unavailable because Inno Setup is missing).
- [x] Check Linux support; add system libraries and CI, local execution unavailable.
- [x] Document hardware-dependent functions and remaining limitations.
- [x] Final review of changes: cross-check new sources, launcher/installer paths, error states and documentation; no changes to main hardware/safety modules.

## Outstanding acceptance checks before a merge / release

- [ ] Complete visual interaction check of the current interface on an accessible Windows desktop.
- [ ] Real hardware testing of motherboard sensors, confirmed fan assignments, session startup/UAC, calibration and verified stop.
- [ ] Test real keyboard reactions and interaction with Classic functions.
- [ ] Successfully run the new Linux CI; check Linux window startup and hardware behaviour.
- [ ] Compile the installer with Inno Setup and run the extended CI installation lifecycle.
- [x] User approval for merging into main granted and local fast-forward completed (October 22, 2026).

## Current status

The Tauri GUI has real IPC for RGB/profile/macro/sensor functions and
the added motherboard, session, channel assignment and keyboard functions.
The existing daemon remains authoritative for hardware access and safety checks.
The GUI branch's backend differences contain custom sensor functions already present
in main; these are not copied over again.

## Test limitations / observations

- Started a real Tauri window with embedded production assets. The accessibility tree
  shows real daemon state (30 fps, effect parameters), with no mock data.
- Windows computer use produces black screenshots here and does not allow reliable interaction checks;
  the first startup attempt reported `GetCursorPos failed: Access is denied (0x80070005)`.
  Visual approval and the complete interactive window check therefore remain outstanding.
- The test process closed with Alt+F4 had exited when checked afterwards;
  the dedicated virtual test daemon continued running (API version 1, approximately 30 fps, 6274 ticks).
  Afterwards, only this test daemon was shut down cleanly through the API.
- Inno Setup is not installed locally. Installer compilation and the installation lifecycle,
  permitted only on temporary CI runners, have not yet been run.
- No real fan/pump/RGB hardware access, UAC approvals or installations
  were triggered for this integration. Linux is not available locally.

## Local test results

- `npm run build` and `npm test`: type checking, production assets and 56 tests passed.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked`: 8 tests passed.
- Tauri Clippy with `-D warnings` and both Rust formatting checks passed.
- Targeted tests for freergb-client, freergb-control, freergb-daemon and freergb-gui,
  including the launcher, passed. The six existing explicit C# interop tests
  remain skipped without their separate fixture invocation.
- `cargo build --workspace --release --locked`: passed.
- `scripts/build-desktop.ps1`: successful Windows release build with embedded
  frontend assets and a copy at `target/release/freergb-desktop.exe`.
- Release smoke test with virtual process plugin: rendering, colour round-trip, plugin crash
  isolation/restart, cooling/sensor/keyboard API and clean shutdown passed.
- Cooling session smoke test: disabled sessions, start/stop errors and invalid
  parameters successfully tested over an isolated real named pipe.
- PowerShell syntax check of the three changed/new build/installer scripts passed.
- `git diff --check`: passed.
