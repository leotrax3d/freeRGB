> **Archived.** Historical record, superseded by [STATUS.md](../STATUS.md) and [HANDOFF.md](../HANDOFF.md). Do not use for current status.

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

Ordered stage 1 — cooling telemetry:
- [x] Strict cached LINK temperatures and explicit unconfirmed child candidates.
- [x] Malformed-data rejection, cache freshness and RPM watchdog regression tests.
- [x] Session ownership and retry/drop restoration without touching foreign sessions.
- [x] 143 Windows tests, fmt/Clippy, release IPC smoke and regenerated installer.
- [ ] SignalRGB closed; fresh live sensor probe and physical/source correlation.
- [ ] Separate process failsafe and bounded duty-command hardware validation.
- [ ] Validated live curve binding. Stage 1 remains open; later stages not complete.

While waiting for hardware availability: corrected installer harness Start Menu
expectation from uploaded runner evidence; full lifecycle rerun pending.


General motherboard follow-up (user clarification, 2026-10-20):
- [x] Keep fan power/PWM/DC separate from motherboard RGB/ARGB providers.
- [x] Optional read-only Windows LHM helper, isolated process and bounded replies.
- [x] Provider prerequisites/status, separate RPM/control candidates, no implicit pairing.
- [x] Additive provider/channel/all-sensor APIs and daemon custom graph integration.
- [x] Classic displays cached state only; no new GUI business logic/config files.
- [ ] Driver-backed runtime validation across motherboard families; verified source labels.
- [ ] Native OS alternatives and optional prerequisite distribution/licensing.
- [ ] Independent failsafe, validated writes and calibration; motherboard RGB headers.
- [ ] Guided optional setup in both GUIs; no mandatory external driver or silent install.

Installer full lifecycle run 39374039676 succeeded. Cross-version upgrades and
interactive manufacturer uninstallers remain unvalidated. Mainboard transport
fixtures exercise no hardware; they do not establish motherboard support coverage.


Mainboard priority continuation:
- [x] Unchecked Setup opt-in for official LHM/PawnIO/missing .NET prerequisites.
- [x] Pinned size/hash and signed-publisher checks, UAC cancellation/error/reboot status.
- [x] Runtime notices/source links and bounded versioned installed-state loading.
- [x] Initial explicit ASUS RGB/ARGB USB adapter; header table and manual ARGB lengths.
- [x] Mock RGB protocol/fallback/ownership tests and real download/staging-only tests.
- [x] Previous commit's CI39447281629 (Windows/Linux) and Package39447319152 passed.
- [x] Read-only broker service candidate, protected-path startup gate and local IPC tests.
- [x] Protected broker installation/updates/removal and isolated SCM acceptance with synthetic worker.
- [ ] Real native worker/driver lifecycle, UAC/reboot and motherboard hardware acceptance.
- [ ] Isolated real prerequisite installation/UAC/reboot acceptance.
- [ ] Live ASUS configuration/color/fallback tests and header setup GUI/configuration.
- [ ] Real mainboard fan writes/calibration/independent watchdog; broader manufacturer support.

Broker boundary continuation:
- [x] Minimal account-specific pipe rights, exclusive persistent handle and server identity checks.
- [x] Identification-only client token, bounded request/reply/ack and cancellation/reconnection tests.
- [x] Monotonic cache aging; expired readings cannot become fresh during forwarding.
- [x] .NET helper/service compilation and no-hardware tests in PowerShell 7/Windows PowerShell 5.1.
- [x] Existing Rust helper transport regression and local installer compilation.
- [x] Commit 0493bd0 passed CI39452690007 and Package39452741550.
- [x] Activate the read-only service through Setup opt-in after protected deployment and SCM acceptance.
  The local boundary tests install no service/driver. Isolated GitHub acceptance
  installs only our service with a compile-time synthetic reader, not a driver.
  Real fan control remains unreleased.

Protected service continuation:
- [x] Embedded helper/archive/notices hashes, immutable source handles and protected staging.
- [x] Fixed owned SCM image/account, user-scoped pipe, standard-user service authentication.
- [x] Update/start-failure rollback and owned removal; foreign owner/tampered helper rejection.
- [x] Mainboard broker run 39469765979 passed real SCM lifecycle and standard-user reads.
- [x] Setup describes optional automatic system service; UAC only for maintenance/system prerequisites.
- [x] Interrupted/failed service removal aborts before deleting the maintenance copy; silent mode never requests UAC.
- [x] Existing API/config formats preserved; protected installation/maintenance state documented.
- [x] Run 39470499850: unauthorized standard-user denial and isolated child-crash recovery.
- [x] Package39470839127 passed, including the actual owned-service removal hook.
- [ ] Real wizard/UAC/driver/reboot, native child crash/hang, power-loss repair and hardware coverage.

Keyboard interaction continuation (user request):
- [x] Daemon-owned per-key glow/fade overlay over any base effect; brightness preserved.
- [x] Explicit K70 CORE USB key/LED map, ISO/ANSI position handling and modifier remaps.
- [x] Optional per-user Windows input helper; no text conversion, event files or input RPC.
- [x] Bounded input queues/age, no-hardware gate, injected-input exclusion and EOF cleanup.
- [x] API: keyboard.reactive.get/configure/preview/disable; existing API types/version unchanged.
- [x] Config: version-1 daemon-owned keyboard-reactive.json; Classic is a thin RPC editor.
- [x] Pure protocol/fade/mapping, virtual frame/brightness, persistence/failure and API tests.
- [x] Native keyboard helper startup/EOF cleanup run39474499240 passed on disposable Windows.
- [ ] Real typing-to-LED acceptance, additional keyboards/layouts, Linux input support.
- [ ] Fresh combined release IPC/installer and CI results for the complete feature.
Mainboard fan/header mapping, validated control and independent failsafe remain
the next cooling work; keyboard support does not enable fan writes.

Channel assignment continuation:
- [x] Pure bounded channel annotation model in freergb-control::channels.
- [x] Explicit control/RPM/device identities; duplicate outputs/RPM/header rejection.
- [x] API: cooling.bindings.get/save; Config: cooling-channel-bindings.json v1.
- [x] Classic RPC-only editor for header, fan/pump role, reported mode and optional RPM.
- [x] Offline/stale/identity/kind resolution without changing existing channel fields.
- [x] Failed persistence preserves active state; annotations never grant write authority.
- [ ] Physical pairing/calibration and independent watchdog before hardware writes.

Keyboard a180dcd: release IPC smoke, Inno compilation/model and workspace Clippy
passed locally; updated native hook lifecycle run39475875921 succeeded.
Complete CI39475875790 and Package39475877313 were dispatched; check their final
results separately from the subsequent channel assignment changes.

Cooling safety continuation:
- [x] Keyboard Package39475877313 passed full release/IPC/installer acceptance.
- [x] Channel annotation CI39476917007 passed Windows and Linux.
- [x] Channel annotation Package39476933684 passed full installer/release acceptance.
- [x] Pure fan response state machine: bounded 100/80/60/40% steps, fresh independent
  samples, applied-command acknowledgment, timing/temperature/stall aborts.
- [x] Pumps excluded; no result until acknowledged 100% restoration, no physical
  mapping/operating-floor claim inferred from response alone.
- [x] Pure expiring duty lease with monotonic issue times, anti-replay and latched
  100% request on expiry/stop/clock fault; no hardware writer added.
- [x] Separate real process/synthetic sink tests for hung/killed producer and
  oversized input; feature-gated fixture excluded from normal build/installer.
- [x] API: cooling.calibration.preview, stateless bounded synthetic trace; no new config.
- [ ] Production independent watchdog/writer integration, authenticated sessions,
  sensor/stall supervision and physical fallback/readback verification.
- [ ] Real fan calibration/restore, driver hangs, service/OS crash and hardware coverage.

These policy/process tests do not enable live motherboard fan control. Existing
read-only service, channel API fields and both GUIs' ownership boundaries remain.

Native adapter continuation:
- [x] Pinned actual LHM Control tests without Computer/SuperIO/driver access.
- [x] Prevent intermediate cached-zero writes during entry to Software mode.
- [x] Reassert identical duties and retry restoration despite cached failed mode.
- [x] Owned acquisition/reservation, conservative fan bounds and pump reduction rejection.
- [x] Partial write/acquisition failure recovery; failed restoration retains ownership.
- [x] Production factory restricted to protected SYSTEM helper/runtime; test-only
  factory absent from production compilation and no activation/RPC setter added.
- [x] Local PowerShell7/5.1 acceptance, helper compilation and broker boundary tests.
- [ ] Authenticated native session/lease transport, cross-process channel ownership,
  independent driver-hang recovery and physical readback before enabling control.

Native-session continuation:
- [x] Fixed-port candidate with random per-session/one-use server-issued tickets,
  exact counters/types/shapes, bounded records and server-clock lease deadlines.
- [x] Private worker factory checks protected SYSTEM helper, SYSTEM pipe ownership,
  registered service PID/config and actual SYSTEM peer token before acquisition.
- [x] Lease/watchdog -> native adapter integration using real LHM/synthetic callbacks.
- [x] Real private pipe tests: waiting client, disconnect, oversized data, latched
  fallback, explicit close and failed recovery; production test entry points absent.
- [ ] Fresh disposable SCM proof of registered service-to-worker client identity.
- [ ] Protected crash journal, worker recovery/readback and physical channel
  verification before production activation. No public setter has been enabled.

Prior native-adapter Package39479651725 SUCCESS, including full installer lifecycle;
native quick workflow39479651926 and broker39479651985 also succeeded.

Recovery continuation while CI ran:
- [x] Native-session CI39481729897, broker39481729758 (real service-client identity)
  and native quick39481729523 succeeded.
- [x] Protected bounded version-1 write-ahead intent outside versioned runtime;
  flushed before native I/O, no low duty replay, exact-ID completion after release.
- [x] Journaled port required by production session/transport factory signatures.
- [x] Process-crash test: killed worker leaves intent and restart cannot reacquire.
- [x] Disk failure, restore failure, clear retry, corrupt/oversized state and real
  LHM/synthetic journal-session-adapter tests in PS7/PS5.1.
- [ ] Fresh SCM proof of protected journal/mutex and denied standard-user write.
- [ ] Automatic checked recovery/readback, multi-channel format/ownership and active
  session coordination with upgrade/uninstall before enabling live fan control.

Service-client continuation during journal CI:
- [x] Journal native quick39483452459 and protected broker39483452345 succeeded,
  including production SYSTEM journal/mutex and standard-user write denial.
- [x] Typed private service client: protected caller/pipe/exact worker PID+image,
  reply identity/counter/duty/state validation, local ticket window and no replay.
- [x] Real pipe tests for set/close/cancel/late ticket and corrupt replies, with
  worker full-speed fallback; PS7/PS5.1 and fixture build pass.
- [ ] Fresh full CI/Package for the complete journal/client increment; production
  worker activation, recovery/readback and maintenance coordination remain open.

Journal baseline554f1c5 now passed CI39483452355, Package39483452206,
broker39483452345 and native quick39483452459. The later typed-client increment
has its own fresh CI/Package pending after commit; do not conflate those results.

## Status update 2026-10-24

Motherboard fan control (up to 16 concurrent live channels, stop all, resume after restart with identity check) and motherboard RGB (ASUS Aura USB all motherboard PIDs, Gigabyte RGB Fusion 2 USB, GUI header configuration, on by default) are no longer experimental; safety mechanisms are unchanged. ASRock Polychrome USB is driven too; MSI Mystic Light is identified only (separate MSI work); SMBus-only boards are not supported. Hardware validation: mock-tested only except CHA_FAN2 single-channel; see `MAINBOARD_RGB.md` and `COOLING_RUNTIME.md`.
