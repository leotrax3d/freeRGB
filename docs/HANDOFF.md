# HANDOFF – current state (2026-10-19)

## Phase
Full-product status is now tracked in `PRODUCT_PLAN.md`: live cooling, macro triggers,
installer acceptance and live updater acceptance are still outstanding. The phase
summary below refers to the RGB prototype, not a complete SignalRGB replacement.
Roadmap phases 1–8 done: first real RGB device (K70 CORE) validated, background
service with local API working end-to-end. Phase 9: native GUI (egui) talks only to the API; declarative and process device plugin foundations implemented. Phase 11 features (profiles, layout editor, audio, screen capture, layered effect editor) are present. Next: hardware soak/platform validation and effect-process plugins.

## Done
* Tool/environment inventory (no "Reverse Engineer Anything" MCP server available).
* SignalRGB 2.5.80 inventory (`REVERSE_ENGINEERING.md`), hardware inventory (`DEVICES.md`).
* Rust 1.99.1 + MSVC Build Tools installed (approved by the user).
* Corsair Bragi protocol implemented and validated on the K70 CORE (`PROTOCOLS.md`).
* Project renamed to freeRGB; private repo `github.com/leotrax3d/FreeRGB`.
* iCUE LINK hub: read-only probe, sensors, lighting of 3 RX fans + Titan AIO + LCD ring, opt-in
  daemon device with RPM watchdog (15-min soak passed).
* `freergbd`: one worker thread per device behind a global HID gate (hub 52 FPS, K70 19 FPS), reconnect with back-off, hot-plug rescan, JSON-RPC over a
  named pipe / Unix socket, clean release to hardware mode on exit; effect/brightness persisted across restarts.

## Working components [confirmed]
| Crate | Content |
|---|---|
| `freergb-core` | Rgb/Hsv, colour correction, LedLayout, Canvas, Frame |
| `freergb-device-api` | `HidTransport`, `RgbDevice`, `VirtualDevice`, `MockTransport` |
| `freergb-effects` | 8 effects (Solid, RainbowWave, Breathing, Gradient, ColorCycle, RadialRainbow, Scanner, Twinkle), serializable `EffectSpec` with defaults |
| `freergb-hid` | hidapi discovery/transport, report descriptor parser |
| `freergb-protocols` | Corsair Bragi adapter (K70 CORE), iCUE LINK hub (probe, lighting, `LinkHubDevice`, `RpmWatchdog`), adapter registry |
| `freergb-daemon` | engine, JSON-RPC API, IPC, hardware scan; binary `freergbd` |
| `freergb-cli` | binary `freergb`: `discover`, `demo-virtual`, `bragi …`, `link-probe`, `link-light-test`, `link-restore`, `ctl` |
| `freergb-client` | blocking JSON-RPC client (used by CLI and GUI) |
| `freergb-audio` | audio analysis (8 log bands, auto gain, smoothing) and system-audio capture (WASAPI loopback) |
| `freergb-screen` | screen capture of the primary monitor (xcap), downscaled to 64x36 |
| `freergb-gui` | binary `freergb-gui` (egui): devices + per-device brightness, canvas layout editor, all effects with parameters, profiles, plugin start/stop, stop daemon |
| `freergb-plugins` | explicit native device processes, versioned stdio, timeouts/fault isolation, virtual Rust reference binary |

## Tests
`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
Current verification results are in `TESTING.md`. GitHub Actions runs fmt/clippy/test/release build on windows-latest and ubuntu-latest (reported green on 2026-10-18, not rechecked this cycle). Hardware on Linux not yet tested.
Hardware results: `DEVICES.md#validation-log`.

## Open issues / questions
* K70 CORE ~19–22 FPS with acknowledged streaming (device-side limit). Pipelining hangs the
  keyboard after up to 2.5 min even with serialised HID access → not usable.
* K70 ~10-min dropout fixed by keepalive ping + plugin pacing; 20-min soak passed (21 FPS).
  Next: multi-hour soak before calling the hub non-experimental.
* Historical observation: while the K70 streams, the hub drops from ~52 to ~28 FPS.
  FIFO HID admission implemented on 2026-10-19; real-device FPS and long-soak impact remain unmeasured.
* Each re-plug of the K70 was needed after a vendor-interface hang; a software reset path
  (e.g. re-open + mode reset) is not known yet.
* `1b1c:1bf0` (no USB interface) – origin unknown.
* Full `key-walk` over all 107 keys not yet visually verified.
* `freergbd` is not installed as a service/autostart (needs user approval: system change).
* SignalRGB, its launcher and service were running on 2026-10-19. They were left
  running; this cycle used virtual devices only.

## Next tasks
1. Validate FIFO HID scheduling with simultaneous K70/hub traffic; measure FPS and run a long soak.
2. Extend process device plugins with evidence-backed hardware implementations and effect-process support (neither supplied yet).
3. iCUE LINK hub: Commander DUO ARGB ports (if any); longer soak (hours) before making `--enable-link-hub` a recommended default.
4. Test hardware access on a real Linux machine with `packaging/linux/60-freergb.rules` (see `INSTALL.md`).
5. Linux GUI/capture validation; autostart proposal (system changes still require approval).

## Entry points
`crates/freergb-daemon/src/main.rs`, `crates/freergb-protocols/src/corsair_bragi/device.rs`,
`crates/freergb-cli/src/main.rs`. SignalRGB plugins (analysis source):
`%LOCALAPPDATA%\WhirlwindFX\SignalRgb\cache\plugin_cdn\main\Plugins`.

## Codex continuation — 2026-10-19
* Initial Git working tree clean at `2e44d1b`; existing implementation preserved.
* Replaced the HID mutex with a dependency-free FIFO ticket gate. All existing
  enumeration/open/read/write call sites keep process-wide serialization.
* Added tests for FIFO admission/re-entry, mutual exclusion, panic release and
  counter wraparound. Blocking HID calls still delay everyone; no hard latency bound.
* Windows checks passed: fmt, workspace Clippy with warnings denied, 97 tests,
  workspace release build, and `scripts/smoke-daemon.ps1` against the release daemon.
* Smoke test confirms real named-pipe requests, virtual-device frame delivery,
  RGB round trip and clean shutdown, with hardware and persistence disabled.
* No new hardware validation or Linux run in this cycle. Prior hardware/CI claims
  above are inherited records, not newly reproduced measurements.
* Entry points for this change: `crates/freergb-hid/src/gate.rs`,
  `scripts/smoke-daemon.ps1`. Plugin stage 2 was addressed in the next cycle below.

## Process-plugin milestone — 2026-10-19
* `WORKLIST.md` tracks the implementation checklist and remaining product backlog.
* New `freergb-plugins`: validated startup manifests, one process per device,
  versioned bounded stdio, read/write deadlines, child termination/reaping, retry
  through existing workers. `freergb-plugin-virtual` is a real Rust child example.
* `--plugin JSON` is repeatable, explicit native-code trust. `plugins.list/start/stop`
  manage only configured IDs; GUI controls added. No implicit plugin discovery.
* `--no-hardware` blocks hardware plugins. At most one hardware plugin; its
  configuration excludes built-in adapters. Existing RGB-app conflict handling applies.
* Effect parameters/layers and saved state/profiles validated; local API lines capped
  at 256 KiB. Invalid profiles do not replace the running effect.
* Windows integration smoke confirms healthy rendering alongside a crashing plugin,
  plugin stop/restart, color round trip and clean daemon exit. No leftover daemon or
  reference-plugin processes observed after the test.
* Native plugins are not sandboxed; forced kill cannot restore real hardware.
  No hardware process plugin, effect-process plugin, hot manifest reload, signatures
  or installation manager is implemented. GUI controls compile; interactive clicks
  were not automated. Linux example/CI configuration added, no local Linux run.
* Entry points: `crates/freergb-plugins/src/lib.rs`, its `tests/process.rs`,
  `examples/plugins/`, `Engine::configure_plugins`, `scripts/smoke-daemon.ps1 -WithPlugin`.

## Cooling/macros/distribution continuation — 2026-10-19
* User expanded scope to cooling, curves, macros, installer, updates and GitHub setup.
  `PRODUCT_PLAN.md` is the full backlog; do not call the entire product complete.
* `freergb-control`: curve validation/interpolation/hysteresis and missing/stale
  sensor fail-safe; bounded macro definitions. No new cooling hardware writes.
* Daemon `Controls`: named curves/macros, session scheduler, delay/repeat/cancel;
  Windows text/key output with `--enable-input-macros` only. Input was not physically
  tested/injected; global triggers/recording/chords remain open.
* New GUI tabs: Cooling (explicit simulation), Macros, Monitoring, Updates.
  LINK monitoring reuses watchdog RPM cache, no new USB reads. Sensor identity stays
  unconfirmed. Sensor age grows even when frames stop.
* `freergb-update`: fixed GitHub release source, stable semver selection, bounded
  authenticated download and SHA-256 verification. Optional background checks/downloads;
  install is user initiated. Private repo has no published release; no live update tested.
* Windows Inno Setup compiled successfully: `target/dist/freeRGB-0.1.0-windows-x86_64-setup.exe`.
  Per-user launcher/shortcuts/uninstall, optional login startup. Setup itself was not run.
  Official Inno 6.7.3 compiler downloaded, Authenticode signature valid (Pyrsys B.V.),
  installed in user scope at `target/tools/inno` with no icons/tasks. Build log:
  `target/installer-build.log`. Pass `-Compiler target/tools/inno/ISCC.exe` to build script.
* Commit `8e76145` pushed; Package run `39284681890` dispatched and last observed queued.
  CI previous baseline: Windows passed; Linux failed missing PipeWire development
  packages. Added those and CI concurrency cancellation; new CI result pending.
* Local Windows: 116 workspace tests pass, Clippy/fmt, debug and release builds,
  expanded real IPC smoke passes (curves/stale preview/RGB macro/peer crash/restart).
  Cached-RPM publication tested with existing hub simulator. No real hardware experiment.
* README heading was edited externally during work (`freeRGB` → `FreeRGB`); preserve
  that working-tree edit and do not attribute or commit it as agent work.

## Parallel follow-up while GitHub queues — 2026-10-19
* Visual macro step editor (add/edit/up/down/delete/repeat) with Advanced JSON fallback.
* `chord` macro steps: canonical modifier order; one ordinary key; all attempted
  presses released in reverse order, including after backend errors. Tests use a
  fake input callback, never foreground input. OS input acceptance remains open.
* GUI histories: at most 128 sensor series × 600 samples; one sample per API snapshot,
  stale/missing/disconnected samples create gaps, absent devices expire after 60 s.
  Memory-only, works across tabs while GUI is open; adds no USB reads.
* `diagnostics.export` returns a redacted report. Monitoring button saves to the
  per-user diagnostics folder. No serials, raw device IDs/names, paths, raw errors,
  capture names or macro/profile contents; no upload. See `DIAGNOSTICS.md`.
* Windows: 120 tests pass, fmt/Clippy, release build and regenerated installer pass.
  Expanded IPC smoke includes report export plus earlier macro/curve/plugin tests.
* Package run `39285464153` for `740b5b8` remained queued at the last check. New source
  changes do not invalidate the already tested local installer. Linux CI still pending.
* Suggested next: foreground-application profiles, explicit global hotkeys, stale/RPM
  alerts; actual cooling control still requires channel identification and validation.

## Client deadline follow-up — 2026-10-19
* Bundled IPC client now has a 3-second total deadline, bounded request/reply sizes,
  strict reply envelope checks and retry of busy Windows pipes. It remains a blocking
  public API, implemented using a private current-thread Tokio runtime. Async callers
  must use `spawn_blocking`. This keeps a stalled daemon from hanging the GUI worker
  forever; multiple queued GUI calls may still delay a full refresh.
* Tests cover silent and nonreading peers, oversized unterminated replies, bad IDs
  and ambiguous replies, plus a real Windows named pipe whose server never replies.

## Session hotkeys and client resilience — 2026-10-20
* Global Windows macro hotkeys: explicit session opt-in, saved bindings, visible
  activation, reserved Ctrl+Shift+F11 cancel, registration rollback and disconnect
  deactivation. No key recording. Native keyboard acceptance remains pending.
* GUI preserves macro/profile action order; only slider setters after the last
  action barrier coalesce. Offline macro starts are discarded; transport failures
  stop the command batch and subsequent optional refreshes. RPC feature errors do
  not prevent other optional queries. API calls have a three-second deadline.
* 128 Windows tests, fmt/Clippy, release build, expanded IPC smoke and installer
  build pass. Logs: target/hotkey-tests.log, target/hotkey-release.log,
  target/hotkey-smoke.log, target/installer-build.log.
* Package run 39285464153 (740b5b8) succeeded. Downloaded artifact SHA256 verified:
  07ac2baebcc74d7333f591e8ddf70835ed5d96e8df6282657a8add1bb82164d4.
  It predates this local installer. No published release/live update acceptance.
* Remote user README/logo commits were fast-forwarded. Preserve the remaining
  unstaged README capitalization edit. A safety stash remains after conflict
  resolution; README is resolved and unstaged.
* Ubuntu CI on a5e55c9 failed missing alsa.pc; added libasound2-dev to CI and
  installation docs. New Linux result pending.

## Advisory sensor warnings — 2026-10-20
* Monitoring supports up to 64 persistent per-device/sensor warning rules in
  sensor-alerts.json. No rules or thresholds are enabled by default. Valid edits
  apply in-memory immediately; Save persists them. See MONITORING.md.
* Min/max limits with recovery hysteresis and three-second condition debounce;
  stale/missing/nonfinite/disconnected/changed-unit readings warn. A snapshot that
  stops advancing for five seconds is unavailable even if cached age_ms looks fresh.
* Global GUI banner links to Monitoring. Acknowledgement lasts only for the current
  condition, not a recurrence after recovery. States reset on startup; missing-device
  rules remain available. Advisory GUI-only; not an independent hardware watchdog.
* 132 Windows workspace tests, fmt/Clippy, release build, virtual IPC/plugin smoke
  and installer compilation pass. Four alert regression tests use fake readings
  and clocks. No real hardware writes, native key injection, GUI click acceptance
  or product installer execution. Logs: target/alert-tests.log,
  target/alert-release.log, target/alert-smoke.log, target/installer-build.log.
* GitHub run 39369935576 (82f1211) passed Windows and Ubuntu fmt/Clippy; remainder
  was still running at the last check. Do not claim overall Linux acceptance yet.

## Foreground application profiles — 2026-10-20
* Windows session-opt-in rules map an executable basename to an existing saved
  profile. Stable foreground observations for two seconds, once-per-entry loading,
  no fallback/restore. Manual profile load and the status pause button stop it.
  Configuration only in application-profiles.json; no activity history.
* Bounded background detection, at most one poll/second. Adapter uses
  active-win-pos-rs 0.11.0; only executable basename retained from its native
  metadata. Linux activation disabled. Native acceptance remains open.
* API worker checks session ticket, current foreground and macro status before
  dispatch. Pause invalidates queued work; already dispatched calls may finish.
  Profile-load acknowledgement revision synchronizes GUI controls after success.
* 136 Windows tests pass, including focus debounce, no repeated loads, re-entry,
  running-macro deferral, missing foreground, invalid rules and stale command
  rejection. Tests do not read actual foreground or write real hardware.
* fmt/Clippy, release build, virtual IPC/plugin smoke and refreshed installer pass.
  Logs: target/app-profile-tests.log, target/app-profile-release.log,
  target/app-profile-smoke.log, target/installer-build.log.
* CI 39370293062 for previous sensor-warning commit was still running. Earlier
  39369935576 was canceled by workflow concurrency; no overall Linux success claim.
  Full behavior and acceptance limits: APPLICATION_PROFILES.md.

## Installer choices, competitor uninstallers and project icon — 2026-10-20
* Setup asks separately about Start Menu, desktop and startup. Start Menu and
  autostart default ON at the user's explicit request; desktop defaults OFF.
  Existing task choices survive upgrades; deselection removes owned entries.
* Optional competitor page scans registered app entries in HKCU/HKLM 32/64 views.
  Supported families: SignalRGB, Razer Synapse/Chroma/Central, Corsair iCUE,
  OpenRGB and Logitech G HUB. Everything unchecked. Only explicit button + confirmed
  product list starts registered manufacturer uninstallers. Next/silent setup do
  not uninstall anything. No actual vendor uninstaller was run during development.
* Strict executable/argument splitting, no shell/PATH/script fallback; ambiguous
  entries direct users to Windows Apps. MSI /I is converted to /X. Entries are
  reread before execution and duplicate registrations are suppressed.
* README logo converted without redesign to ICO (16–256) and PNG. Shared Windows
  resources embedded in CLI, daemon, launcher and GUI; window and Setup icons use
  the same asset. Regenerate with scripts/build-icons.py (Pillow). Resource extraction
  from local release executables verified the project icon.
* New daemon.shutdown_if accepts expected_executable; only a canonical path match
  permits shutdown. Installer helper stop-installed-daemon --directory PATH uses
  it for upgrade/uninstall, preserving other checkouts. Old daemons must be stopped
  manually. Profile/config directory is never removed by setup/uninstall.
* 137 Windows tests, fmt/Clippy, release build, guarded IPC/plugin smoke, Inno
  compilation and compiler-built Pascal model tests pass. No product install on
  the personal computer. Model harness exits before installing and never runs an
  uninstaller. Full lifecycle script refuses local personal-machine execution.
* Package workflow now runs fresh install, same-version upgrade path/task removal,
  unrelated-daemon preservation and uninstall with saved-profile sentinel on an
  ephemeral Windows runner. Runner result is required before claiming acceptance;
  cross-version update, GUI wizard/vendor dialogs and signing remain untested.

## Ordered stage 1 — LINK cooling telemetry — 2026-10-20
* User requested cooling → installer → updates → macros → monitoring → RGB/profiles.
  Stage 1 is not complete. SignalRGB and launcher remained running at the last
  process inventory. User said they would close them and report back; that closure
  has not yet been confirmed. No live USB probe, duty write or mode change was run.
* Strict type/length/count telemetry decoding; signed tenths Celsius, bounded
  plausibility, cached temperature publication alongside RPM. Failed temperature
  reads preserve sample age. Candidate slot+1 child labels remain unconfirmed.
* Require spinning baseline before software session. Hardware ownership tracked
  separately from connection status; failed restores retried during disconnect/drop;
  never restore a foreign software session. This is cooperative cleanup, NOT an
  independent watchdog process. Cooling API remains simulation-only.
* Evidence and remaining live gates in COOLING_VALIDATION.md. Upstream protocol
  facts pinned to OpenLinkHub c8c1d2a99a56aa1d907e43137d127c2048f41ba5; installed
  plugin hash recorded there. No REA server used and no GPL code copied.
* 143 Windows tests, fmt/Clippy, release build, virtual IPC/plugin smoke and Inno
  installer build pass. New tests cover malformed telemetry, signed/invalid temps,
  no I/O from cache reads, temperature failures without lost RPM protection,
  failed-restore retry/drop and foreign-session preservation.
* Logs: target/cooling-tests.log, target/cooling-release.log,
  target/cooling-smoke.log, target/installer-build.log.
* Package run 39371595581 failed a harness assertion: it expected a custom Start
  Menu group, but setup uses its configured default with the group page disabled.
  Uploaded log shows successful creation under Programs/freeRGB and autostart.
  Corrected the harness to test that actual configured group; fresh run pending.
  Do not describe the remaining lifecycle as accepted yet.


## General mainboard provider and GUI compatibility — 2026-10-20

- User clarified general motherboard fan power/PWM/DC plus separate RGB/ARGB
  coverage for future users. Prefer native OS APIs; no mandatory external driver.
  Writing our own kernel driver still requires signing/installation/maintenance.
- Custom Celsius graph evaluation/history now runs in daemon/control, not egui.
  Additive get/save/sources/values API, version-1 graph in existing control.json;
  GUI daemon-shutdown action removed. Tauri paths and shared API types unchanged.
- Optional isolated Windows LHM0.9.6 reader: provider/channel/sensor cache APIs,
  fixed read-only capabilities, unconfirmed physical mapping/PWM/DC mode. Includes
  our helper only in Setup. LHM/PawnIO not bundled, installed, started or elevated.
- Launcher forwards explicit --mainboard-library only when starting a new daemon;
  demo rejects it. Existing normal shortcuts/startup remain launcher-based.
- IMPORTANT: stock LHM SuperIO Close requests fan-control resets. Dedicated helper
  never acquires software fan control and exits without invoking Computer.Close;
  Windows reclaims process handles/memory. Native behavior still needs validation.
- Hardware-independent transport tests cover missing library, synthetic ready,
  malformed/oversized replies, early exit, hang/deadline and bounded process cleanup.
  No driver-backed LHM live scan, fan write or calibration was performed.
- Package run 39374039676 succeeded, superseding the previous failed Start Menu
  assertion. Cross-version update/signing/vendor-uninstaller acceptance remain open.
- Fresh short LINK read-only probe with software window and subsequent mode-1
  verification succeeded; candidate RPM/temperature mapping remains unconfirmed.
  A real daemon cache-acceptance harness did not connect to the hub in its deadline;
  it cleaned up and hardware mode was verified. Do not claim live cache acceptance.
