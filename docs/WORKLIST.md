# Implementation worklist — 2026-10-19

**Full product plan:** `PRODUCT_PLAN.md`. The milestones below cover implemented
subsystems, not completion of the full application. Cooling hardware, macro triggers,
installer acceptance and live update acceptance remain tracked there.

Follow-up completed while GitHub queues:
- [x] Visual macro step editor and bounded chord steps, with error cleanup tests.
- [x] Cached sensor histories with stale gaps, expiry and memory bounds.
- [x] Redacted local diagnostic export, API and GUI save action.
- [x] 120 Windows tests, release build, installer rebuild and expanded IPC smoke.

Suggested next: profiles by foreground application, global macro triggers, and
sensor/watchdog notifications. Full product acceptance still follows `PRODUCT_PLAN.md`.

Completed milestone: process-plugin foundation and host hardening.
This milestone does not imply completion of all hardware/platform goals.

- [x] Versioned process protocol, validated opt-in manifests, bounded I/O,
      timeout/crash isolation and child cleanup.
- [x] Rust example plugin and subprocess integration tests.
- [x] Daemon integration, plugin diagnostics and lifecycle API; GUI start/stop controls.
- [x] Effect and persisted-profile validation, including layered effects.
- [x] Bounded local API input with regression tests.
- [x] Windows fmt/Clippy/109 tests/release and real IPC smoke checks, including peer crash.
- [x] Update API/plugin/security/testing/handoff documentation and commit.

Subsequent acceptance/backlog (not silently marked complete):
- Simultaneous K70/LINK hub FIFO throughput measurement and multi-hour soak.
- Linux build/GUI/capture verification and real Linux hardware test.
- Further device protocols only after evidence; no guessed commands.
- Effect-process plugins and broader hot-load UX after the device-plugin foundation.
- Service/autostart installation requires approval for system configuration.

Ordinary reversible implementation decisions are autonomous. Firmware, drivers,
irreversible actions and system-wide changes retain the original approval boundary.

Completed follow-up — 2026-10-20:
- [x] Bounded IPC client with total deadline and malformed-response tests.
- [x] Session-opt-in Windows macro hotkeys and reserved cancellation shortcut.
- [x] Preserve queued action order; stop refreshes after connection failure.
- [x] 128 Windows tests, Clippy/fmt, release IPC smoke and regenerated installer.
- [x] Verify successful GitHub Package artifact; add missing Ubuntu ALSA dependency.

Remaining acceptance: native hotkeys/input, installer lifecycle, live updater,
Linux runner confirmation and identified/validated cooling hardware channels.

Completed follow-up — advisory sensor monitoring:
- [x] Persistent min/max sensor warning rules, hysteresis and debounce.
- [x] Missing/stale/disconnected/changed-unit warnings and snapshot freshness guard.
- [x] Cross-page banner, acknowledgement and recurrence handling.
- [x] 132 Windows tests, Clippy/fmt, release smoke and regenerated installer.
- [ ] Interactive monitoring acceptance with identified real channels.

Completed follow-up — foreground application profiles:
- [x] Saved Windows executable/profile rules with per-session opt-in.
- [x] Background foreground detection, stability timer and no repeated loads.
- [x] Pause/queued-work invalidation, macro deferral and confirmed-load UI sync.
- [x] 136 Windows tests, Clippy/fmt, release IPC smoke and regenerated installer.
- [ ] Interactive Windows focus-switch acceptance; Linux support.

Completed follow-up — installer:
- [x] Independent Start Menu, desktop and startup choices; startup defaults ON.
- [x] Optional manufacturer uninstallers for recognized RGB products, all unchecked.
- [x] README project icon in Setup, application executables and GUI window.
- [x] Scoped graceful shutdown; leave daemons from other directories alone.
- [x] 137 Windows tests, Pascal model tests, fmt/Clippy, release smoke and installer.
- [x] Ephemeral-runner install/reinstall/uninstall/profile-preservation harness.
- [ ] Fresh lifecycle runner result, wizard/vendor interaction and cross-version update.
