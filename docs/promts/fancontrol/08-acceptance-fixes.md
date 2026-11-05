# Prompt 08 — Fix the findings of the first owner acceptance (2026-11-05)

Read `00-common-rules.md` first. Work on the existing branch
`cooling/fix-reader-hang` (already contains fixes 1 and 2 below plus the
acceptance record); open one PR to `main` when done.

Evidence: `docs/COOLING_ACCEPTANCE.md`, section "Owner acceptance session
2026-11-05". Owner board: ASUS PRIME X870-P WIFI, NCT6701D; CHA_FAN2 =
`/lpc/nct6701d/0/control/2` (rpm `fan/2`); AIO pump = `control/1` (rpm `fan/1`),
bound as role `pump`. No hardware writes in this prompt — everything must be
reproduced in tests/simulators; owner re-test happens afterwards (prompt 07).

## Already fixed on the branch (verify, add tests)
1. `44781c2` LHM storage enumeration blocked the protected reader's first
   snapshot; storage disabled; kill watchdog when a worker reply is late.
   Add a C# fixture test for a worker that never answers (must end in
   "reader failed", not "starting").
2. `79c2972` Daemon mainboard provider never retried after a non-ready reply
   (daemon starts before the broker). Retries every 5 s now. Add a Rust test
   (fixture helper returns `permission_denied` first, then `ready`).

## To fix
3. **Broker pipe stuck busy after setup.** After installing over a running
   install, `\\.\pipe\freergb-mainboard-v1` (single instance) stayed
   `ERROR_PIPE_BUSY` until `Restart-Service freeRGBMainboard`. Find the client or
   server path that keeps the instance connected (look at `BrokerService.Serve`,
   `ReadSnapshotRequest`/`ReadAcknowledgment` with `AwaitIo` on synchronous vs.
   overlapped handles, daemon/helper restarts during setup). Fix so a stuck
   client can never block the instance (bounded per-client deadline that always
   disconnects, or a supervised server loop). Add an SCM/fixture test.
4. **Fine calibration sweep ends after one step.** `cooling.sessions.start`
   `{"mode":"calibration","sweep":{"step_percent":5,"lowest_percent":40},...}`
   on a verified channel completed in ~10 s with `minimum_applied_percent: 95`,
   `report.calibration: null`, nothing stored. Trace `ControlMode::Sweep` through
   `cooling_sessions.rs` -> native session -> report/storage
   (`cooling-fan-support.json`); fix and add an end-to-end test against the
   NCT6701D simulator that asserts all 13 steps, the stored map and `rpm_range`.
   Also check that plain `mode:calibration` (100/80/60/40) stores its result.
5. **Unexplained jobs.** The daemon showed jobs 1–4 (2× curve, 2× calibration at
   100 %) before any request was sent. Find who starts sessions at daemon start
   (resume of remembered assignments, desktop guided setup, profiles) and make
   it visible (job `origin` field in `cooling.sessions.status`) and documented.
6. **Pump not detected.** After upgrade no channel had a role; the owner had to
   say which header is the pump. Guided setup (prompt 05) must require a role
   for every channel with a tachometer before any reduction and show unassigned
   channels as "100 % (role unknown)". Verify this in the Tauri setup flow.
7. **Installer UX.** A second installer started while the first waited behind
   UAC. Use a mutex/`AppMutex` so only one setup runs.

8. **Reader flapping** (owner evening session 2026-11-05). With the kill
   watchdog the steady 3 s reply deadline became real; slow snapshots got the
   worker killed in a loop (14/24 ready with iCUE running or LHM GPU off,
   20/24 with iCUE closed); R5 ended "Cooling supervision stopped: Stopped,
   Some(Stale)". Fixed: motherboard tree first and on every snapshot, other
   hardware on a measured cadence with conservative ages, slow updates named
   in the provider message, broker deadlines 30 s first / 6 s steady, daemon
   6 s. Simulated slow-sensor test.
9. **Maintenance drain refused with an active background curve** (D6):
   setup failed "Mainboard-Rückgabe nicht bestätigt" while control/2 ran at
   47 %. Fixed: background cooling returns to firmware before the drain and
   restarts if the drain is refused.
10. **Duplicate channel binding**: guided setup added a second CHA_FAN2
    binding in channel-id spelling next to a plain-id one. Fixed: ids are
    canonicalized server-side, a legacy twin is merged, canonical duplicates
    are rejected; the desktop upsert treats both spellings as one header.

## Done when
All ten items have tests (simulated), CI green, PR open; the acceptance record
lists the fix commits; owner re-test steps (A1 after upgrade, B1) are written in
`07-hardware-acceptance.md`.
