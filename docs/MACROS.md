# Macros

The daemon manages named macro definitions in `control.json` next to saved state.
GUI: Macros tab, visual step editor with add/edit/reorder/delete and advanced JSON,
save/run/cancel/delete. Triggers are GUI/API requests and session-opt-in global
shortcuts on Windows. Both macro pages include a focused recording pad.

```json
{"repeat":1,"steps":[{"type":"color","color":{"r":255,"g":0,"b":0}},{"type":"delay","ms":1000},{"type":"brightness","value":0.5}]}
```

Steps: `color`, `brightness`, `effect` (EffectSpec), `delay` (ms), `text` (text),
`key` (key), `chord` (keys). Example shortcut: `{"type":"chord","keys":["ctrl","c"]}`.
Chords require 1–4 distinct modifiers (`ctrl`, `alt`, `shift`, `meta`) and exactly
one ordinary key (a–z, 0–9 or enter/escape/tab/space/backspace/arrows). Modifiers
are pressed in canonical order and every attempted press is released in reverse
order even on an input backend error. OS-level failures can still prevent release;
the macro stops and reports the error. Recorded `key_down`/`key_up` steps use the
portable chord vocabulary and must be balanced. Text, click and chord steps cannot
run while a recorded key is held.

Supported single keys: enter, escape, tab, space, backspace, up/down/left/right,
media_play_pause, volume_up, volume_down, volume_mute. No shell/program execution.
Max 128 steps, 1–20 repeats, 10 s per delay, 60 s aggregate delay, 1024 bytes per
text step. One macro at a time; cancellation stops subsequent steps. Each render
tick executes at most one step. OS text/key output may take additional time.

Windows text/key injection uses Enigo 0.6.1 and requires daemon
`--enable-input-macros`. It sends to the foreground application. Add a delay to
switch focus before execution. Tests never inject real input. Linux text/key
execution is currently rejected; RGB macros work through the shared engine.

## Global shortcuts (Windows)

In Macros → Global macro shortcuts, add a binding to an existing saved macro,
save the bindings and explicitly enable them for the current GUI session.
Each binding uses Ctrl plus Alt and/or Shift with F6–F10. Ctrl+Shift+F11 is reserved
for macro cancellation. Duplicate/reserved bindings and missing macro names are rejected;
an OS registration conflict rolls back the partial registration. Debounce is 500 ms.

Bindings are saved in the per-user `hotkeys.json`; activation is never persisted.
The GUI shows a persistent active indicator. Closing it unregisters the shortcuts;
daemon disconnect also disables them and requires explicit reactivation. Deleting
a bound macro disables shortcut handling when that shortcut is next encountered.
Text/key output still requires the daemon opt-in flag. Hotkeys do not record keys.

The native manager is created on the GUI event-loop thread using global-hotkey 0.8:
https://docs.rs/global-hotkey/0.8.0/global_hotkey/
Backend registration and actual shortcut events require manual Windows acceptance;
automated tests use a fake registration backend and never grab real shortcuts.

## Focused recording

Choose **Record** in either macro page, then type into its pad. Key mode captures
portable presses/releases with timing; text mode captures the pad's reviewed text
as one step at Stop, preserving composition and edits rather than typing cadence.
The Classic pad captures Ctrl/Alt/Shift modifier changes and
uses F12 to stop and review. Unsupported keys are ignored. Mouse recording is not
enabled.

Switching window, page or field (including password fields) discards the buffer.
Hidden webviews stop recording. Secure desktop is covered by focus loss where the
GUI reports it; there is no global text hook or password-field inspection in other
applications. A 30-second daemon deadline and a 3-second focus heartbeat deadline
independently discard the buffer if the GUI freezes, disconnects or closes. Native
focus/secure desktop delivery still needs owner acceptance.

Autorepeat presses are ignored. Adjacent text bursts within 20 ms coalesce up to
1024 bytes. Delays use daemon monotonic receipt time and split into at most
10-second steps. The 128-step limit reserves space to release held keys. Invalid
events or overflow discard the whole buffer. Stop closes held pairs and returns
a draft; **Save macro** is a separate explicit operation. Nothing is written to
the library during recording. Status/tray replies never include keys or text.
Cancel/error/end of playback drops the persistent Windows Enigo backend, which
attempts held-key release; OS failures cannot guarantee physical release.

### Recording RPC contract

Calls use the existing local JSON-RPC transport. The caller chooses a unique
`session` string (1–128 bytes). Competing sessions are rejected; only the matching
session can send events or stop. This prevents accidental client races and is
not an additional local-user security boundary.

| Method | Parameters | Result |
| --- | --- | --- |
| `macros.record.start` | `{"session":"unique-client-id"}` | true; rejected during playback or another recording |
| `macros.record.event` | `{"session":"...","event":{"type":"key_down","key":"a"}}` | true; also accepts `key_up`, `text`, `heartbeat`, `privacy_stop` |
| `macros.record.stop` | `{"session":"...","discard":false}` | validated `{repeat:1,steps:[...]}` draft; discard returns null |
| `macros.record.status` | none | recording flag, step count, reason and scope; no content |

Unknown fields/events and invalid keys are rejected as invalid parameters.
`macros.status` additionally exposes `recording`. The tray adds a red recording
dot, menu line and **Recording macro (unsaved)** tooltip on its normal poll; both pads show an immediate
indicator. Recording and playback are mutually exclusive.

## Device macro keys (fixture-tested; physical identity follow-up)

Both macro pages assign saved macros to F6–F10 exposed by the cached protocol key
map, including the existing K70 CORE map. Typing/modifier keys and cancellation
F11 are reserved; unmapped keys, missing macros and duplicate (device id, usage)
assignments are rejected. Bindings persist atomically in `device-macros.json`.
Enabling is a separate daemon-session opt-in and is never persisted. Saving
disables activation. The desktop panel disables it on unmount; Classic activation
lasts for the daemon session, as labelled in its checkbox.

| Method | Parameters | Result |
| --- | --- | --- |
| `macros.device_keys.get` | none | bindings, enabled state, cached maps, `physical_identity_pending:true` |
| `macros.device_keys.save` | `{bindings:[{device_id,usage,macro_name}]}` | true; checks saved macro and current protocol map |
| `macros.device_keys.enable` | `{enabled:true}` | true; disallowed under no-hardware input policy |

Dispatch requires an exact device id, connected mapped target and a fresh event
(maximum age 250 ms). It ignores anonymous events, wrong devices, key repeats,
modified F-keys, missing/deleted macros and offline targets; debounce is 500 ms.
Playback retains its normal input opt-in. No API method injects a synthetic device
trigger into the live engine.

The shared Rust `keyboard::InputEvent` contains `usage`, `age_ms`, `pressed` and
optional `device_id`. Windows helper release events carry `pressed:false`; older
press records default to true. Optional identity defaults to None, preserving
reactive lighting. **The current Windows low-level hook emits null device ids,
so physical device macros remain inactive.** Read-only Raw Input identity
correlation to the protocol inventory needs a separate implementation and fixtures;
never infer the source from a selected RGB target or anonymous scan code. Linux
capture can use the same event type when its source identity is available (WS3).
Pure dispatch/API fixtures prove policy, not live attribution. This limited
delivery follows the coordinator's explicit fallback decision.

## Owner acceptance steps

1. On an isolated test desktop, exercise both pads with press/release, modifiers,
   composed text, delays, 128-step overflow and 30-second timeout. Review a stopped
   draft; verify it is absent after restart until explicitly saved.
2. Switch page/window/field, minimize, close/disconnect/freeze the GUI and invoke
   secure desktop while recording. Confirm no content is returned after a privacy
   stop and the tray indicator clears on its next poll. Password input in any other
   application must be absent from drafts.
3. With explicit Windows output opt-in, accept output/repeats/cancellation and
   held-key cleanup in a harmless editor. Automated verification never injects
   native input on the development computer.
4. Complete and fixture-test Raw Input source-to-inventory correlation before
   claiming physical triggers. Verify identical F6 keys on two keyboards trigger
   only the assigned device; modified F6 remains a global shortcut; injected output
   cannot recurse; unplug/replug/deletion/debounce work; activation resets after
   daemon restart.

The Automation STATUS milestone remains unchecked because physical per-device
input and native owner acceptance are pending.
