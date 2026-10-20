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


### Motherboard RGB and installer state

No RPC methods or existing wire types change in this increment. An explicitly
opted-in ASUS `0b05:19af/ff72/00a1` RGB controller appears through the existing
`devices.list`/layout/effects/brightness APIs as a `LightingController`. Its LED
layout is discovered during worker connect: fixed slots plus manually configured
ARGB lengths. CLI activation and bounds (four 0..512 LED counts, supported firmware
headers, fallback limitations) are in `MAINBOARD_RGB.md`. Normal scanning leaves
this experimental adapter unopened. No fan-control setter is introduced.

Installer-created `mainboard-provider.json` and `mainboard-install-result.json`
live in the application installation directory, version 1, documented in
`MAINBOARD_INSTALLATION.md`. They are not API parameters or GUI config files;
clients must not execute their paths or interpret them as permission to elevate.
The existing mainboard provider still reports missing rights/prerequisites through
its status. API version remains 1; Tauri paths/dependencies are unchanged.

The internal read-only broker (`MAINBOARD_BROKER.md`) retains this same
helper envelope and these existing statuses. It adds no RPC methods, setters,
GUI configuration files or wire fields. Relayed samples retain their original
age. Setup's mainboard opt-in also deploys the protected sensor service; no GUI
or daemon elevation is needed for its local cached readings. Provider status
still reports missing hardware/prerequisites. Protected installation/maintenance
diagnostic formats are documented in `MAINBOARD_INSTALLATION.md`; no new GUI
config file or API parameter selects an executable, library or service identity.


For concurrent Windows integration tests only, daemon `--ipc-instance NAME`
selects `\\.\pipe\freergb-NAME`; NAME is 1..64 ASCII letters/digits/hyphens and
requires **both** `--no-hardware` and `--no-state`. Normal server/client endpoint,
DACL, remote-client rejection and API version stay unchanged. This hidden test
option does not redirect either GUI or the SDK. The smoke harness connects to its
own random instance and never shuts down a GUI preview's daemon.

### Keyboard reactions (additive, API version 1)

* `keyboard.reactive.get` takes no parameters and returns `{config,status,devices}`.
  It reads caches only. `devices` contains keyboard objects with `id`, `name`,
  `connected:bool`, `keys` (USB keyboard usage decimal string -> layout LED index)
  and the existing `LedLayout` shape as `layout`. An empty key map means no verified
  key/LED mapping, not a promise of support. Existing `devices.list` is unchanged.
* `keyboard.reactive.configure` takes `{config:ReactiveConfig}` and returns `true`
  after validation and saving. Config: `version:1` (default if omitted),
  `enabled:bool`, `color:Rgb`, `fade_ms:50..5000`, `iso:bool`, `devices:string[]`.
  Other fields are required; unknown config fields are ignored. At most 64 unique
  nonempty IDs, each <=1024 UTF-8 bytes/no control characters; serialized config
  <=128 KiB. Unsupported versions/types/bounds and save failures return `-32602`;
  old configuration remains active on configure failure. Unknown/disconnected
  target IDs may be saved but do not start capture without a mapped connected
  keyboard. No native I/O or hook runs in the RPC handler.
* `keyboard.reactive.preview` takes `{usage:u16}` and returns `true` for USB keyboard
  page usages 4..115 or 224..231. Other values/types return `-32602`. It enqueues a
  short lighting pulse on configured targets without enabling input capture. A
  successful enqueue does not guarantee that a target is connected/mapped. No
  base effect or configuration is changed.
* `keyboard.reactive.disable` takes no parameters and returns
  `{stopped:true,persisted:bool,error:string|null}`. It immediately clears runtime
  input/pulses even if saving fails. `persisted` is false under `--no-state` or
  a write failure; an error explains a failed save, and next startup may still
  load the previously enabled file. No new RPC error code is introduced.

`status` includes `state`, `error:string|null`, `scope`, `records_text:false`.
States: `disabled`, `no_targets`, `blocked_no_hardware`, `unsupported_platform`,
`waiting_for_keyboard`, `starting`, `listening`, `faulted`, `stopped`. Capture is
off by default, Windows-only, per-user, and combines all local keyboards while
lighting only selected targets. `--no-hardware` blocks native input capture;
preview remains available. Input events are not exposed through RPC or stored.
Daemon-owned `keyboard-reactive.json` version 1 and all lifecycle/limitations are
documented in `KEYBOARD_REACTIVE.md`. No GUI business logic, `EffectSpec`,
`DeviceSummary`, `DeviceStatus`, `Diagnostics` or `SensorReading` change. Classic
edits sliders locally and sends configure on Apply; no new slider setter is added
to `is_setter()`.

### Motherboard channel annotations (additive, API version 1)

* `cooling.bindings.get` has no parameters and returns
  `{config,assignments,error:string|null,persistence_enabled:bool}`. Config is
  `{version:1,bindings:ChannelBinding[]}`. Each assignment contains `control_id`,
  `state` (`unavailable`, `identity_mismatch`, `incompatible_channel`, `stale`,
  `user_assigned`), `hardware_verified:false`, `writable:false`.
* `cooling.bindings.save` takes `{config}` and replaces the complete set, returning
  `true`. Each binding has `provider:"mainboard.lhm"`, `device_id`, `control_id`,
  optional nullable `tachometer_id`, `header`, `role:"fan"|"pump"`, and optional
  `electrical_mode:"unknown"|"pwm"|"dc"` (default `unknown`). Unknown fields are
  ignored. Version and other fields are required. Maximum 128 bindings/256 KiB;
  nonblank device ID <=1024 UTF-8 bytes, channel IDs <=4096, trimmed nonempty header
  <=64 bytes; all text rejects control characters. Controls and assigned RPM
  channels are unique per provider, header names unique per device using Unicode
  lowercase. Control and RPM must match the specified device and advertised kinds/
  capability in cached inventory. Missing/stale references may be saved offline.
  Invalid input or available identity/kind/capability conflicts: `-32602`;
  persistence failure: `-32000`. Either leaves active configuration unchanged.

Get resolves cached readings only; missing values/age >3000 ms yield `stale`
after identity/kind/availability checks. User annotations never enable writes,
calibrate hardware, change PWM/DC or alter existing channel fields. Daemon-owned
`cooling-channel-bindings.json` version 1, startup failures, full schema and state
precedence are documented in `COOLING_CHANNEL_BINDINGS.md`. Under `--no-state`
saves are in-memory only. Classic uses explicit Save; no new `is_setter()` entry.

### Fan calibration policy preview (additive, API version 1)

`cooling.calibration.preview` takes `{role:"fan",critical_c:number,samples:[]}`.
Role `pump` is explicitly rejected; critical temperature must be finite 40..100
Celsius. Each sample has required unsigned `at_ms`, `rpm_age_ms`,
`temperature_age_ms`; optional nullable `applied_sequence:u64`, `rpm:number`,
`temperature_c:number`. Maximum 512 samples, each `at_ms` <=61001, relative to
initial time 0. Unknown fields are ignored. Invalid types/limits/role/critical
temperature return `-32602`. Sensor range/freshness faults and reversed clocks
instead produce simulated restoration/failure output. No sensor input is treated
as a live measurement and no sample or result is persisted.

Returns `{initial,trace,final_output,simulated:true,hardware_control:false}`. Each
output has `phase`, `sequence`, `duty_percent`, nullable `reason`, nullable `result`.
Phases: `applying`, `settling`, `sampling`, `restoring`, `completed`, `aborted`,
`restoration_failed`. Result (only after successful restoration):
`{response_observed:bool,lowest_tested_percent:40,suggested_floor_percent:50|null,
hardware_verified:false}`. Empty/truncated traces remain initial/incomplete;
there is no background session. Full duty sequence, acknowledgment/sample timing,
abort conditions and the limited meaning of a floor suggestion are specified in
`COOLING_SAFETY.md`. Existing `cooling.status` continues to deny hardware writes.
No setter, GUI-process logic, shared configuration or existing wire type changes.

The version-pinned native control adapter candidate in `MAINBOARD_NATIVE_CONTROL.md`
adds no callable setter or activation flag. Protected helper construction is gated
and no production entry point invokes it. Existing provider/channel read-only
fields, `cooling.status`, API version and configuration formats remain unchanged.
