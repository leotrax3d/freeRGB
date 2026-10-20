# Local API

Implemented in `crates/freergb-daemon/src/{api,ipc}.rs`, API version **1**.

## Transport
* **Windows:** named pipe `\\.\pipe\freergb`. Remote clients are rejected; the default
  pipe DACL gives write access only to the creating user, LocalSystem and administrators.
* **Linux/Unix:** socket `$XDG_RUNTIME_DIR/freergb.sock` (fallback: temp dir), mode `0600`.
* No network listener.

The bundled blocking client caps requests at 256 KiB, replies at 16 MiB, and uses
a 3-second total connect/write/flush/read deadline. Windows busy pipes are retried
inside that deadline. Reply version/ID and result/error structure are validated.
Async callers must use `spawn_blocking` for the blocking API.

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
| `daemon.shutdown_if` | `{expected_executable:absolute_path}` | `true` and shutdown only for a canonical executable-path match; `false` leaves another copy running; malformed relative/missing paths produce `-32602` |
| `devices.list` | – | `[{info, status, brightness, region, leds, frames, dropped, last_error}]` |
| `cooling.status` | – | Hardware-control availability and reason (currently preview only) |
| `cooling.sensors.get` / `cooling.sensors.save` | – / `{graph:{version:1,nodes:{...}}}` | Read/validate/save the custom Celsius source graph; evaluation and bounded history run in the daemon. See `CUSTOM_SENSORS.md`. |
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

### Custom sensor readings

`cooling.sensors.sources` (no parameters) returns an object mapping native source
keys to display labels. Keys are JSON-encoded `[device_id,sensor_id]` strings.
Only connected cached Celsius sources are included; presence does not guarantee
a fresh value. It performs no synchronous hardware reads.

`cooling.sensors.values` (no parameters) returns `{now_ms,values:{name:reading}}`.
Each reading is `{value:number|null,sampled_at_ms:number|null,error:string|null}`.
Timestamps use the daemon's monotonic sensor epoch, not Unix time. The daemon
rechecks freshness on every values request; sources older than 3000 ms become
unavailable. Time averages continue while the GUI is closed. See
`CUSTOM_SENSORS.md` for all graph bounds, calculations and reset behavior.

`cooling.sensors.save` accepts graph version 1 (omitted version defaults to 1).
Unsupported versions, invalid graphs and persistence failures return the existing
invalid-parameters error `-32602`; saved definitions remain unchanged. Empty graphs
are valid. Methods are additive; existing API types and version remain unchanged.

### Optional motherboard provider (additive)

All three methods take no parameters (extra parameters are ignored, like the other
list getters), read caches only and never start a provider, request elevation,
install/start a driver or issue hardware writes:

* `cooling.providers.list` returns an array of provider summaries. Currently this
  covers the optional `mainboard.lhm` provider, including when disabled; it is not
  an exhaustive list of existing RGB/HID adapters. Fields: `id`, `name`, `status`,
  `message`, `read_only:true`, `control_enabled:false`, `rgb_supported:false`,
  `age_ms:number|null`. Status values: `disabled`, `starting`, `ready`,
  `missing_library`, `missing_driver`, `permission_denied`, `no_supported_hardware`,
  `faulted`, `stale`. `age_ms` is elapsed time since the cached request began.
* `cooling.channels.list` returns cached mainboard fan-RPM and control-percentage
  channels. Each item: `provider`, `id`, `device_id`, `name`, `kind` (`fan` or
  `control`), `reading` (unchanged `SensorReading` shape), `control_available:bool`,
  `writable:false`, `physical_mapping:"unconfirmed"`, `electrical_mode:"unknown"`.
  `id` is a JSON-encoded `[native_hardware_id,native_sensor_id]` string. A control
  object does not establish safe writes or pairing with an RPM sensor. No setter
  is exposed. Missing/unavailable providers return an empty array.
* `monitoring.sensors.list` returns cached readings across existing RGB devices and
  the optional provider. Each item: `provider` (`device` or `mainboard.lhm`),
  `device_id`, `device_name`, `sensor` (unchanged `SensorReading`). Native mainboard
  device IDs are prefixed `mainboard.lhm:`; sensor IDs retain upstream identity.
  Supported mainboard units are `°C`, `RPM`, `%`. Disconnected existing RGB devices
  return null sensor values. Mainboard readings older than 5000 ms return null
  values with their increasing age; faults clear the mainboard inventory. Existing
  `devices.list`, `SensorReading` and `Diagnostics` are unchanged.

Mainboard Celsius readings also join `cooling.sensors.sources` and derived graph
inputs; the graph's stricter 3000 ms freshness bound still applies. API version
remains 1. There are no new method-specific error codes; startup/prerequisite
failures are represented in provider status, not fabricated sensor readings.
Internal helper replies are bounded to 256 KiB, 64 hardware nodes, 1024 sensors,
512-byte nonempty IDs/names and unique hardware/sensor identities. Values must be
finite and in -100..200 °C, 0..100000 RPM or 0..100 percent respectively. Invalid
replies fault the provider. These sanity limits are not safe operating limits.
See `MAINBOARD_SUPPORT.md` for prerequisites, timeouts, lifecycle and limitations.
