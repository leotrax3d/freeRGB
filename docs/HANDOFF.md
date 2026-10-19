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
