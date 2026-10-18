# Local API

Implemented in `crates/freergb-daemon/src/{api,ipc}.rs`, API version **1**.

## Transport
* **Windows:** named pipe `\\.\pipe\freergb`. Remote clients are rejected; the default
  pipe DACL gives write access only to the creating user, LocalSystem and administrators.
* **Linux/Unix:** socket `$XDG_RUNTIME_DIR/freergb.sock` (fallback: temp dir), mode `0600`.
* No network listener.

## Protocol
JSON-RPC 2.0, one request per line, one response per line (UTF-8, `\n`).

```
→ {"jsonrpc":"2.0","id":1,"method":"color.set","params":{"color":"#00ff00"}}
← {"jsonrpc":"2.0","id":1,"result":true}
```

CLI client: `freergb ctl <method> [params-json]`.

## Methods
| Method | Params | Result |
|---|---|---|
| `daemon.version` | – | `{api_version, daemon_version}` |
| `daemon.shutdown` | – | `true`; daemon releases devices and exits |
| `devices.list` | – | `[{info, status, brightness, region, leds, frames, dropped, last_error}]` |
| `diagnostics.get` | – | `{uptime_s, ticks, target_fps, measured_fps, effect, brightness, devices, conflicts, audio}` – `conflicts` lists other RGB software that is running (hardware is released meanwhile) |
| `effects.list` | – | effect names: `solid`, `rainbow_wave`, `breathing`, `gradient`, `color_cycle`, `radial_rainbow`, `scanner`, `twinkle`, `audio_pulse`, `audio_spectrum` (audio effects start the system-audio capture while active) |
| `effects.default` | `{"name": "scanner"}` | default `EffectSpec` for that effect (use it as a template for `effects.set`) |
| `effects.get` | – | current `EffectSpec` |
| `effects.set` | `EffectSpec`, e.g. `{"type":"rainbow_wave","speed":0.3,"scale":1.0}`, `{"type":"breathing","color":{"r":0,"g":155,"b":222},"period_ms":3000}` | `true` |
| `color.set` | `{"color":"#rrggbb"}` | `true` (sets a solid effect) |
| `brightness.set` | `{"value": 0.0..=1.0}` | `true` |
| `devices.set_region` | `{"id": "…", "x": 0.0, "y": 0.0, "w": 0.5, "h": 1.0}` | `true`; places the device on the effect canvas (normalised, persisted) |
| `devices.set_brightness` | `{"id": "hid:1b1c:1bfd:…", "value": 0.0..=1.0}` | `true`; per-device brightness, multiplied with the global one, persisted |
| `profiles.list` | – | sorted profile names |
| `profiles.save` | `{"name": "…"}` | `true`; saves current effect + brightness |
| `profiles.load` | `{"name": "…"}` | `true`; applies and persists the profile |
| `profiles.delete` | `{"name": "…"}` | `true` |

## Errors
`-32700` parse error, `-32600` missing method, `-32601` unknown method,
`-32602` invalid params (also invalid profile names), `-32004` profile not found,
`-32003` profiles disabled (`--no-state`), `-32000` storage I/O error.

Profiles live in `profiles/<name>.json` next to the state file. Names: 1–64 characters,
letters, digits, space, `-`, `_` – nothing that could leave the directory.

## Planned
`layouts.get|set`, `plugins.list`,
`frames.push` (screen ambience / external plugins), event subscription for the GUI.
