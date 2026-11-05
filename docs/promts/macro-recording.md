# Prompt — Macro recording and per-device macro keys

Independent of fan control and of OpenRGB device ports. Read `docs/HANDOFF.md`,
`docs/STATUS.md` (Automation section), `docs/MACROS.md`,
`docs/KEYBOARD_REACTIVE.md`, `docs/LOCAL_API.md` and `docs/SECURITY.md` first.

## Rules
- Branch `macros/recording` from `main`, own git worktree, one PR to `main`;
  delete the branch after merge. Extend the existing macro engine, editor and
  hotkey code — no second implementation.
- Do not touch cooling code (`crates/freergb-control` cooling modules,
  `crates/freergb-daemon/src/cooling_*`, `providers/mainboard`) or `modules/`
  (other people work there).
- No UAC/elevation, no installer runs, no hardware writes; the owner is away.
  Real keyboard acceptance steps go into a checklist for later.
- Privacy: recording is opt-in, visibly active (GUI + tray indicator), stops on
  focus loss/timeout, never records while a password field or the secure
  desktop is active where detectable, stores nothing until the user saves.
- Repo language English; chat with the user in German.

## Tasks
1. **Recording**: record key down/up, text and mouse buttons (optional) with
   delays into the existing macro step format; coalesce delays, cap length;
   start/stop via API (`macros.record.start|stop|status`), Tauri editor and
   Classic GUI button. Windows via the existing optional keyboard helper;
   Linux: document as unsupported or implement via evdev if already used.
2. **Per-device macro keys**: assign macros to device keys that the protocol
   exposes (start with the K70 CORE mapping already used for reactive
   lighting; G-keys/extra keys where available), plus global shortcuts as today.
   Validation: no duplicates, reserved keys refused.
3. Tests: recorder state machine, coalescing, privacy stops, API validation,
   GUI unit tests with mock data (no real brand names in public demo data).
4. Docs: `MACROS.md`, `LOCAL_API.md`; update the STATUS line with evidence but
   do not check "real native macro output/hotkey acceptance".
5. Owner checklist (real typing on the K70, recording, playback) at the end of
   `MACROS.md`.

## Done when
Feature works against mocks/fixtures, CI green, PR open, owner test steps
written down.
