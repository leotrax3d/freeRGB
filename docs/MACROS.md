# Macros

The daemon manages named macro definitions in `control.json` next to saved state.
GUI: Macros tab, editable JSON, save/run/cancel/delete. Current triggers are explicit
GUI/API requests; recording and global hotkeys are not implemented.

```json
{"repeat":1,"steps":[{"type":"color","color":{"r":255,"g":0,"b":0}},{"type":"delay","ms":1000},{"type":"brightness","value":0.5}]}
```

Steps: `color`, `brightness`, `effect` (EffectSpec), `delay` (ms), `text` (text),
`key` (key). Supported keys: enter, escape, tab, space, backspace, up/down/left/right,
media_play_pause, volume_up, volume_down, volume_mute. No shell/program execution.
Max 128 steps, 1–20 repeats, 10 s per delay, 60 s aggregate delay, 1024 bytes per
text step. One macro at a time; cancellation stops subsequent steps. Each render
tick executes at most one step. OS text/key output may take additional time.

Windows text/key injection uses Enigo 0.6.1 and requires daemon
`--enable-input-macros`. It sends to the foreground application. Add a delay to
switch focus before execution. Tests never inject real input. Linux text/key
execution is currently rejected; RGB macros work through the shared engine.
