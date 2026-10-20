# Macros

The daemon manages named macro definitions in `control.json` next to saved state.
GUI: Macros tab, visual step editor with add/edit/reorder/delete and advanced JSON,
save/run/cancel/delete. Triggers are GUI/API requests and session-opt-in global
shortcuts on Windows. Recording is not implemented.

```json
{"repeat":1,"steps":[{"type":"color","color":{"r":255,"g":0,"b":0}},{"type":"delay","ms":1000},{"type":"brightness","value":0.5}]}
```

Steps: `color`, `brightness`, `effect` (EffectSpec), `delay` (ms), `text` (text),
`key` (key), `chord` (keys). Example shortcut: `{"type":"chord","keys":["ctrl","c"]}`.
Chords require 1–4 distinct modifiers (`ctrl`, `alt`, `shift`, `meta`) and exactly
one ordinary key (a–z, 0–9 or enter/escape/tab/space/backspace/arrows). Modifiers
are pressed in canonical order and every attempted press is released in reverse
order even on an input backend error. OS-level failures can still prevent release;
the macro stops and reports the error. No standalone modifier/held-key steps.

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
