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
| `cooling.status` | – | Hardware-control availability and reason (currently preview only) |
| `cooling.curves.list` | – | Map of curve names to definitions |
| `cooling.curves.save` / `cooling.curves.delete` | `{name,curve}` / `{name}` | Validate/store or delete curve |
| `cooling.preview` | `{curve,temperature_c?,age_ms?}` | `{duty_percent,simulated:true}`; no hardware write |
| `macros.list` | – | Map of named macro definitions |
| `macros.save` / `macros.delete` | `{name,definition}` / `{name}` | Validate/store or delete |
| `macros.run` / `macros.cancel` | `{name}` / – | Start saved definition or cancel subsequent steps |
| `macros.status` | – | `{running,input_enabled,error}` |
| `plugins.list` | – | `[{id,name,access,enabled,allowed,status,last_error,frames}]`; status null while stopped or awaiting start/conflict resolution |
| `plugins.start` / `plugins.stop` | `{"id":"example-strip"}` | `true`; only IDs explicitly configured with `--plugin`; start occurs on next render tick |
| `diagnostics.get` | – | `{uptime_s, ticks, target_fps, measured_fps, effect, brightness, devices, conflicts, audio, screen}` – `conflicts` lists other RGB software that is running (hardware is released meanwhile) |
| `diagnostics.export` | – | Offline report version 1 with anonymous device/plugin indexes and fault flags; omits IDs/serials, paths, names, raw errors and user macro/profile contents. See `DIAGNOSTICS.md`. |
| `effects.list` | – | effect names: `solid`, `rainbow_wave`, `breathing`, `gradient`, `color_cycle`, `radial_rainbow`, `scanner`, `twinkle`, `audio_pulse`, `audio_spectrum`, `screen_ambience`, `layers` (audio/screen effects start the respective capture only while active) |
| `effects.default` | `{"name": "scanner"}` | default `EffectSpec` for that effect (use it as a template for `effects.set`) |
| `effects.set` with `{"type":"layers","layers":[…]}` | each layer: `{"effect": EffectSpec, "blend": "normal|add|multiply|screen|max", "opacity": 0..1, "region": {x,y,w,h}?}`, 1–8 layers, no nesting | custom effect from the editor (validated, `-32602` on errors) |
| `custom_effects.list` | – | names of saved custom effects |
| `custom_effects.save` | `{"name": "…", "effect": EffectSpec}` | `true` (validated) |
| `custom_effects.get` | `{"name": "…"}` | the saved `EffectSpec` |
| `custom_effects.delete` | `{"name": "…"}` | `true` |
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
`-32003` profiles disabled (`--no-state`), `-32000` storage I/O error,
`-32005` plugin missing from allowlist or hardware start disabled.

Requests are limited to 262144 bytes including newline. An oversized request receives
`-32600` with null ID and the connection closes. Invalid UTF-8 or incomplete EOF lines
also close the connection. Other connections remain usable.

Effect parameters are validated before mutation, including inside layers: speed
-100..100, scale 0.001..1000, period_ms 1..86400000, angle -360000..360000,
scanner width 0.001..1, twinkle density 0..1, audio sensitivity 0..100, screen
saturation 0..10. Floating parameters must be finite. State/profile loads apply
the same validation, plus brightness and device region validation.

Profiles live in `profiles/<name>.json` next to the state file. Names: 1–64 characters,
letters, digits, space, `-`, `_` – nothing that could leave the directory.

## Planned

`devices.list` also contains cached `sensors` entries (`id,name,unit,value,age_ms`).
LINK publishes the RPM samples already read by its watchdog, with unconfirmed
channel identities explicitly labelled. No extra transport requests. Age continues
to increase if frame delivery stops. See `COOLING.md` and `MACROS.md` for validation
and current execution limits. New cooling/macro validation/storage errors currently
use `-32602` and an explanatory message.
`layouts.get|set`,
`frames.push` (screen ambience / external plugins), event subscription for the GUI.
