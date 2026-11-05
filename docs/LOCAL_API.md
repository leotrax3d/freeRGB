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
| `cooling.curve_set.get` / `cooling.curve_set.save` | – / `{set:{nodes:{name:{type,...}}}}` | Read/validate/save the composite curve set (`graph`, `constant`, `trigger`, `mix`, `follow`; cycle-checked). Not bound to fan channels. |
| `cooling.curve_set.values` | – | `{now_ms,outputs:{name:{percent,failsafe}},simulated:true}`: read-only evaluation against live sensor readings and custom sensor nodes; missing/stale/critical inputs give 100 % with `failsafe` and propagate through mix/follow. |
| `cooling.curves.save` / `cooling.curves.delete` | `{name,curve}` / `{name}` | Validate/store or delete curve |
| `cooling.preview` | `{curve,temperature_c?,age_ms?}` | `{duty_percent,simulated:true}`; no hardware write |
| `cooling.sessions.status` | – | Experimental live session `{available,job_id,state,report,error}`; independent of legacy preview `cooling.status` |
| `cooling.sessions.start` | `{mode,control_id,temperature_id,rpm_id,confirmed_fan,curve?,critical_c?,duty_percent?,composite_output?,target_rpm?}` | `{job_id}`; asynchronous UAC authorization, then protected native acquisition |
| `cooling.sessions.stop` | `{job_id:u64}` | `true` means stop queued; poll status for verified return |
| `macros.list` | – | Map of named macro definitions |
| `macros.save` / `macros.delete` | `{name,definition}` / `{name}` | Validate/store or delete |
| `macros.run` / `macros.cancel` | `{name}` / – | Start saved definition or cancel subsequent steps |
| `macros.status` | – | `{running,input_enabled,error}` |
| `plugins.list` | – | `[{id,name,access,enabled,allowed,status,last_error,frames}]`; status null while stopped or awaiting start/conflict resolution |
| `plugins.start` / `plugins.stop` | `{"id":"example-strip"}` | `true`; only IDs explicitly configured with `--plugin`; start occurs on next render tick |
| `modules.status` | – | Setup state: `{first_run, cards:[{key,name,vendor,kind,phase,module,download_size,progress,error,configured,dismissed,builtin,installed,connected,engine_id,technical}], catalog, job, installed, trusted_release_key, light_test_remaining_s}`. See [hardware modules](HARDWARE_MODULES.md) |
| `modules.catalog.refresh` | – | `true`; fetches and authenticates the catalog in the background |
| `modules.setup.start` / `modules.setup.retry` | `{"keys":["usb:1532:00a3:if0:0001:0002"]}` | `true`; runs in the service (retry first stops that device's module) |
| `modules.setup.cancel` / `modules.setup.complete` | – | `true`; cancel aborts downloads, complete ends the first-start setup |
| `modules.setup.dismiss` / `modules.forget` | `{"keys":[...]}` | `true`; dismiss hides new devices, forget removes a device from the configuration |
| `modules.rollback` | `{"id":"razer-chroma"}` | `{"version":"0.1.0"}`; returns to the last working or previous version |
| `modules.light_test` | `{"devices":[engine ids]}` | `true`; one soft 2.4 s white pulse, then the running effect continues |
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

`modules.*` returns -32006 when the service runs with `--no-modules`.

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

`cooling.sensors.catalog`, `cooling.file_sensors.get|save` and `cooling.gpu.list|set|release`
are documented in `COOLING_SOURCES_GPU.md`.

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
  `age_ms:number|null`, `board:null|{manufacturer,product,bios_version}` (public
  SMBIOS strings, at most 128 characters each). Status values: `disabled`, `starting`, `ready`,
  `missing_library`, `missing_driver`, `permission_denied`, `no_supported_hardware`,
  `faulted`, `stale`. `age_ms` is elapsed time since the cached request began.
* `cooling.channels.list` returns cached mainboard fan-RPM and control-percentage
  channels. Each item: `provider`, `id`, `device_id`, `name`, `kind` (`fan` or
  `control`), `reading` (unchanged `SensorReading` shape), `control_available:bool`,
  `writable:false`, `physical_mapping:"unconfirmed"`, `electrical_mode:"unknown"`,
  `sensor_id` (raw native sensor identity, as used by sessions), `hardware_name` and
  `support:null|{tier,register_readback,software_mode,held_full}` for controls.
  `support.tier` is the chip's capability from the protected inventory:
  `verified` (audited register readback) or `lhm_supported` (LHM software mode, no
  readback; see `COOLING_BROAD_SUPPORT_PLAN.md`). `software_mode:true` means LHM
  already reports software control (another tool); sessions refuse it.
  `held_full:true` means a crashed lower-tier worker's channel is held at 100%
  until reboot. `support:null` comes from older protected services (verified-only).
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

The later private session/worker pipe candidate (`MAINBOARD_NATIVE_SESSION.md`)
also adds no public RPC method or activation flag. Its single-use tickets, lease
and protected service-client identity are internal to the native worker design;
they must not be exposed through either GUI. Production helper still serves only
snapshots and no production caller invokes the candidate.

The write-ahead recovery candidate adds protected metadata
`%ProgramFiles%\freeRGB\mainboard-control-recovery.json` version 1, documented in
`MAINBOARD_CONTROL_RECOVERY.md`. This is privileged runtime state, not a shared
GUI configuration or public RPC parameter. No API method/field/error code or
activation flag changes. Production session factory now requires the journaled
port; the whole candidate remains unreachable from the snapshot CLI/service.

## Experimental protected motherboard sessions (additive, 2026-10-22)

`cooling.status` and all previews retain their existing behavior. Live control is
available only through the separate `cooling.sessions.*` methods. The daemon must
run on Windows with `--mainboard-library DIR`, without `--no-hardware`; the bundled
helper and current protected service must be installed. `available=true` reports
worker enablement, not verified service/hardware support. No session is persisted
or replayed at startup. The current native journal permits one channel/job only.

**Support tiers.** The daemon resolves the tier from the protected inventory
(`support`) and the user's channel binding, never from the request:
`verified` chips keep the legacy authorization (unchanged record, register
readback mandatory). `lhm_supported` chips may run `calibration` (100 -> 40%) but
reduced-duty curves only after a passed guided channel test (see the plan).
Channels bound as pumps, or with `software_mode`/`held_full`, are refused here.
Lower tiers pass `tier` and `role` into the UAC authorization; the protected
worker re-validates the floor (40% fan, 100% pump, 0% only when unrestricted).

### Start parameters

`cooling.sessions.start` rejects unknown fields. Required fields:

| Field | Validation |
|---|---|
| `mode` | Exactly `curve` or `calibration` |
| `control_id` | `/lpc/CHIP/INSTANCE/control/INDEX`; index 0..6 |
| `temperature_id` | `/lpc/CHIP/INSTANCE/temperature/INDEX`; index 0..255 |
| `rpm_id` | `/lpc/CHIP/INSTANCE/fan/INDEX`; index 0..6 |
| `confirmed_fan` | Must be `true`: user physically confirms a normal fan and matching tachometer; pumps excluded |

IDs are at most 128 ASCII alphanumeric, slash or underscore characters. CHIP must
be nonempty; INSTANCE parses as u16. All three IDs must name the same native
hardware parent. This is syntax validation only: native authorization/binding also
validates the actual supported chip and existing source identities. It never
infers a physical header or CPU/coolant identity from a sensor label.

For `mode=rpm`, `target_rpm` (positive integer) is translated by the daemon into a
fixed duty from this channel's measured duty/RPM pairs: the latest completed
calibration for the current board/BIOS/channel identity, else its passed channel
test (never mixed). Interpolation is linear between measured points; targets at or
above the fastest measurement run at 100%; targets below the slowest measurement
are refused (no extrapolation towards a stall). The resulting duty then runs
exactly like `mode=manual` (tier floor, critical override, supervision); a duty
below the floor is refused. It is open loop: the measured RPM is reported but not
used to correct the duty. `cooling.support.get` channels carry
`rpm_range:{min_rpm,max_rpm}` when a usable map exists. RPM assignments are
remembered and re-resolved from the current measurements on resume.

For `mode=composite`, `composite_output` names a node of the saved composite
curve set (`cooling.curve_set.*`) and `curve` must be absent; `critical_c`
(40..100 C, default 80) applies to the session's own `temperature_id`. The daemon
evaluates the set every second and feeds that duty to the session; a missing,
stale (> 3 s) or fail-safe output, a stale session temperature or reaching
`critical_c` gives 100%. The tier floor still applies, pumps are refused, and the
protected worker's lease supervision returns the channel to firmware (or holds
100% on legacy verified workers) if the daemon stops feeding it. Composite
assignments are remembered and resumed like curve/manual ones. They run only as
daemon-driven live sessions, never in the boot-time background service.

For `mode=curve`, `curve` is required and `critical_c` must be absent/null. Curve
fields are `points:[{temperature_c,duty_percent}]`, `minimum_percent`, `critical_c`,
`hysteresis_c`. There must be 2..16 points; temperatures -20..110 C strictly
increase; duty never decreases and each duty is minimum..100%. Saved curves allow
a minimum of 0..100%; native execution requires at least the channel's tier floor
(40% unless unrestricted), critical 40..100 C, hysteresis 0..10 C, optional
`max_rise_percent_per_s`/`max_fall_percent_per_s` 1..100 (step limits that never
delay the 100% override for invalid sensors or critical heat), and the last point cannot exceed
critical temperature. All numbers must be finite. Native authorization rounds the
floor up and critical threshold down to whole units, conservatively.

For `mode=calibration`, `curve` must be absent/null; optional `critical_c` defaults
to 80 C and must be 40..100 C. The bounded Rust calibration uses a 40% floor.
The resulting minimum recommendation does not assert hardware verification.
Neither mode accepts GUI-provided measurements or arbitrary native commands.

Example (explicit user confirmation required before sending):

```json
{"jsonrpc":"2.0","id":10,"method":"cooling.sessions.start","params":{"mode":"calibration","control_id":"/lpc/nct6701d/0/control/2","rpm_id":"/lpc/nct6701d/0/fan/2","temperature_id":"/lpc/nct6701d/0/temperature/1","confirmed_fan":true,"critical_c":80}}
```

`{job_id:u64}` acknowledges queueing only. Each start launches the bundled helper
which requests UAC via the installed protected maintenance executable. It writes
short-lived, protected fan/source-specific authorization (four minutes); normal
installation alone grants none. The daemon authorizer times out after 125 seconds.
The service/worker perform independent native checks, telemetry/watchdog supervision
and write-ahead recovery. Unsupported binding, stale sensors, stalled RPM or
critical heat cannot be overridden by GUI assertions.

### Status and stop

`cooling.sessions.status` takes no parameters and returns:

- `available:bool`, `job_id:u64|null` (daemon-lifetime identity).
- `state`: `disabled`, `idle`, `authorizing`, `executing`, `stopping`, `completed`,
  `stopped`, `failed`, `faulted` or `blocked`.
- `tier:null|"verified"|"lhm_supported"|"unrestricted"` of the current/last job.
- `report:null|{calibration,native_closed,maintenance_authorized,minimum_applied_percent,initial_rpm,last_rpm,tier,return_evidence,fault,warning}`.
  `calibration` is null for curves, otherwise the calibration result. RPM values
  are initial/last checked native readings; `minimum_applied_percent` is the
  lowest acknowledged command, not a mechanically verified minimum. `tier` is
  what the protected worker admitted (null: legacy verified worker).
  `return_evidence` is `register_readback` (verified), `firmware_callback` (lower
  tiers: owned SetDefault callback plus cleared journal, no register proof) or
  `none`. `fault`/`warning` explain a faulted run or an allowed fan stop.
- `error:string|null`: asynchronous authorization, transport or runtime failure.

`completed`/`stopped` after native execution require the authenticated firmware
return and durable journal-clear proof (`maintenance_authorized=true`). A stop
during authorization can yield `stopped` with no report because no acquisition was
started. `failed` covers authorization/pre-start failure. `faulted` (lower tiers
only) means supervision or the worker ended the run and the channel was returned
to its captured firmware mode with a cleared journal; new starts remain possible.
`blocked` means uncertain
native execution/return: inspect protected recovery state before daemon restart;
new starts are refused. No failed outcome is evidence of successful return.

`cooling.sessions.stop` requires `job_id` as u64 matching the active job. It returns
`true` when queued, not when released. It may be repeated while active. During
UAC authorization, stop waits for the authorization helper to finish before
preventing acquisition; it does not dismiss the Windows prompt. Runtime stopping
allows 20 seconds for checked return. Status polling remains available meanwhile.
Closing either GUI does not stop the daemon/session. Daemon teardown closes the
command channel and joins the dedicated worker, requesting checked native return.

### Errors

- `-32602`: malformed/missing/wrongly typed required fields or unknown start fields;
  missing/non-u64 stop `job_id`.
- `-32006`: semantic validation failure, confirmation missing, provider disabled,
  active/blocked job, exhausted job identity, worker/queue unavailable, or stop ID
  not active. Synchronous errors do not queue native acquisition.
- Once a start is queued, UAC refusal/cancellation/timeout, unavailable or obsolete
  installed service, invalid native source/binding and execution/return faults are
  reported through `status.error` and its state, not a later RPC response.

Legacy clients need not use these methods; API version remains 1. Hardware and
release acceptance limits are recorded in [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md).

## Background fan control (additive, 2026-10-22)

Saved curves run in the protected SYSTEM service from Windows startup, without a
GUI or daemon (`COOLING_BROAD_SUPPORT_PLAN.md`, work package 4).

* `cooling.background.get` -> `{available, saved:{channels}, apply:{state,error},
  service}`. `saved` is the last request (per-user config
  `cooling-background.json`). `apply.state`: `idle`, `applying`, `applied`,
  `failed`. `service` is the status the protected service publishes in
  `%ProgramFiles%\freeRGB\mainboard-control-status.json`
  (`{version:1,error,updated_utc,channels:[{control_id,tier,state,duty_percent,fault,warning}]}`)
  or `null`; it is display data only.
* `cooling.background.apply` `{channels:[{control_id,temperature_id,rpm_id,curve,confirmed_fan}]}`
  (at most 8, unknown fields rejected) -> `true` when the hand-over started. Each
  channel is resolved like `cooling.sessions.start` (`mode=curve`): tier from the
  protected inventory, 40% fan floor unless unrestricted, LHM-supported reduction
  only with channel-test evidence. The helper asks for UAC and the service's
  `--configure-cooling` re-validates and writes the protected
  `mainboard-control-channels.json`, then reloads (service command 130). An empty
  list disables background control.

## Guided channel test and channel evidence (additive, 2026-10-22)

* `cooling.sessions.start` accepts `mode:"channel_test"` (same fields as
  `calibration`, fans only). It runs 100 -> 70 -> 50% (authorized floor 50%, never
  below 40%) under calibration supervision and returns the channel to firmware.
  `cooling.sessions.status` adds `mode` and `control_id`; the calibration report
  adds `tested:[{duty_percent,rpm}]` and `monotonic_response`.
* A completed test is stored by the daemon (`cooling-fan-support.json`), bound to
  board manufacturer/product, BIOS version, hardware and control identity. It
  passes with a monotonic response (each lower duty >= 5% lower RPM, all >= 100
  RPM). A passed test allows LHM-supported curves down to 40% and supplies the
  RPM bands for background plausibility. Any identity change makes it `stale`.
* `cooling.support.get` -> `{unrestricted_enabled, board_identity_available,
  channels:{CONTROL_ID:{test,tested,unlock}}, error}`; `test` is `none`, `passed`,
  `failed` or `stale`.

## Unrestricted unlock (additive, 2026-10-22)

* `cooling.support.settings {unrestricted_enabled:bool}` -> `{background_reapplied}`.
  Global switch, default off. Turning it off disables every unlock; saved
  background channels are re-validated and those no longer permitted are dropped
  (new UAC hand-over).
* `cooling.channels.unlock {control_id, confirmation}`: requires the switch, a
  reported board identity, a controllable channel not in foreign software mode,
  and `confirmation` equal to the channel name. Stored bound to board, BIOS and
  channel (`cooling-fan-support.json`); invalid after any change.
* `cooling.channels.confirm_pump {control_id, confirmation}`: second, separate
  confirmation for a channel bound as pump, `confirmation` = `PUMP <name>`.
* `cooling.channels.revoke {control_id}` -> `{revoked, background_reapplied}`.

An unlocked channel runs as tier `unrestricted`: no channel test needed, curves
down to 0% (fan stop), RPM deviations and stalls are warnings. Pumps leave 100%
only after the pump confirmation, and are never calibrated or tested. Always
active regardless: firmware return on stop/crash/telemetry loss/watchdog,
critical-temperature override to 100%, refusal of foreign software mode.

`cooling.sessions.start` also accepts `mode:"manual"` with `duty_percent:0..100`
(no curve, optional `critical_c`, default 80): a flat curve with the same floors
(40% unless unrestricted), critical-temperature override and supervision.

## Controller, curve and calibration parameters (additive, 2026-11-04)

All fields are optional and additive to `cooling.sessions.start`; requests
without them behave as before. Unknown fields are still rejected. Every mode
keeps lease, tier floor, critical-temperature override and fail-safe return.

* `mode:"linear"` with `linear:{min_c,max_c,min_percent,max_percent,critical_c,hysteresis_c?}`
  runs a FanControl linear curve as a validated two-point graph (`mode:"curve"`
  rules and floors). Remembered like curves.
* Curves (`curve`, background channels, composite `graph` nodes) accept 2..64
  points and `interpolation:"linear"|"step"` (omitted = linear; `step` holds the
  lower point's duty up to the next point).
* `mode:"rpm"` with `closed_loop:true` (optional `pi:{kp,ki,max_correction_percent}`,
  defaults 0.01 / 0.02 / 20): PI correction on top of the measured duty/RPM map.
  The integral is bounded and does not wind up against 0/100 %. Missing, stale
  (>1.5 s) or stalled RPM telemetry runs open loop (map duty) and clears the
  integral; a stale or critical temperature gives 100 %. Without `closed_loop` the
  previous open-loop behavior is unchanged.
* `mode:"calibration"` with `sweep:{step_percent:2..20,lowest_percent,probe_stall?}`
  runs 100 % down to `lowest_percent` in fixed steps (2 s settle, 5 samples each).
  `lowest_percent` below 40 requires `probe_stall:true`, which is accepted only on
  unrestricted fan channels: a stopped fan is recorded as `stall_percent` and the
  sweep probes upwards (at most five steps) for `start_percent`. The report's
  calibration result carries `stall_percent`/`start_percent`; they are stored with
  the channel identity in `cooling-fan-support.json` (`calibrations`) and shown in
  `cooling.support.get` channels. Never on pumps; restore to 100 % before completion.
* `shaping` (curve, manual, linear, composite and RPM modes; refused for
  identify, channel test and calibration):
  `{offset_percent, min_percent, max_percent, response_delay_ms (≤60000),
  hysteresis_up_percent, hysteresis_down_percent (0..50), avoid:[{from,to,unit?}],
  stop_below_percent (1..50), spin_up:{percent,ms (≤10000)}}`. Order: fail-safe
  input → 100 % (no other step); offset, then min/max; hysteresis (a change of at
  least the threshold is a candidate) and delay (a candidate applies after being one
  continuously for the delay, elapsed `>=` delay); fan stop below the threshold
  (only unrestricted fans, never pumps); tier floor; resonance avoidance (an output
  strictly inside a range moves to the nearer boundary, ties and boundaries below the
  floor go up; overlapping ranges merge); spin-up boost when leaving 0 % or the
  stored stall duty. `unit:"rpm"` ranges are converted with the channel's measured
  map at start; without one the request is refused.
* Composite curve sets accept `linear` nodes (`sensor,min_c,max_c,min_percent,
  max_percent,critical_c`) and `equilibrium` nodes (`sensor,target_c,critical_c,
  min_percent,max_percent,gain_percent_per_c=1,max_change_percent_per_s=2,trend_s=10`):
  duty changes by `gain*(T - target + trend_s*dT/dt)` per second, bounded by the
  change rate, starting at `max_percent`; a missing/stale/critical sensor fails safe
  and restarts at `max_percent`. Both are cycle-checked and propagate fail-safe like
  other nodes.

## Cooling profiles (additive, 2026-10-22; complete configurations 2026-11-04)

Named, switchable complete cooling configurations (per-user config
`cooling-profiles.json`, at most 32 profiles; file version 1 still loads). A
profile holds `channels` (background curves, at most 8), optional `assignments`
(live channels: `cooling.sessions.start` requests of continuous modes),
`curve_set` and `sensors`. Channel bindings are never part of a profile. See
[COOLING_SERVICE.md](COOLING_SERVICE.md).

* `cooling.profiles.list` -> `{profiles:[{name,channels,live_channels,curve_set,sensors}], active, error}`.
* `cooling.profiles.get {name}` -> the profile object.
* `cooling.profiles.save {name, profile? | channels?}` -> `true`. Names are 1..64
  characters without control characters or surrounding spaces. `profile` stores
  a complete profile, `channels` only a background set (version 1 behaviour);
  without either the current complete configuration is captured. Everything is
  validated; saving never touches hardware.
* `cooling.profiles.delete {name}` -> `{deleted}`.
* `cooling.profiles.activate {name}` ->
  `{profile, started:[control_id], restarting:[control_id], background_applied}`.
  Every channel is re-resolved under the current policy first; one channel no
  longer permitted fails the whole activation before anything changes. Live
  channels outside the profile return to firmware, unchanged ones keep running.
  Background curves are handed to the protected service (UAC) only when they
  differ from the saved set.
* `cooling.profiles.export {name}` ->
  `{schema:"freergb.cooling-profile", version:2, name, source:{board, devices}, profile}`.
* `cooling.profiles.import {document, name?, remap?}` -> `{name, report:{name,
  remapped, same_board, unresolved, needs_confirmation}}`. `document` is the
  exported object or its JSON text; `remap` maps source device prefixes to local
  ones (`{"/lpc/it8689e/0":"/lpc/nct6701d/0"}`, automatic with one device on
  each side). On another board or after a remap every `confirmed_fan` is
  cleared.

## Cooling service (additive, 2026-11-04)

* `cooling.service.get` -> `{mode:"login"|"service", boot_profile, headless,
  boot:{state:"pending"|"resumed"|"profile"|"failed"|"unavailable", ...}, error}`.
* `cooling.service.set {mode, boot_profile}` -> `true` (per-user
  `cooling-service.json`). `headless` is true only in a process started with
  `freergbd --cooling-service`, which applies the boot profile (service mode)
  or the remembered assignments and runs the independent watchdog.

## Cooling CLI

`freergb cooling <command> [--json]` calls the API above; `--json` prints the
daemon's result unchanged for scripts, otherwise a short text summary. Exit code
is non-zero on any daemon error.

| Command | API |
|---------|-----|
| `list` | `cooling.channels.list` |
| `status` | `cooling.sessions.list`, `cooling.service.get`, `cooling.profiles.list` |
| `set-manual <control_id> <percent> --temperature <id> --rpm <id> [--confirmed-fan]` | `cooling.sessions.start` (`mode:"manual"`) |
| `stop <control_id>` / `stop-all` | `cooling.sessions.stop_channel` / `stop_all` |
| `profiles` / `save <name>` / `apply <name>` / `delete <name>` | `cooling.profiles.*` |
| `export <name> [--out file]` | `cooling.profiles.export` |
| `import <file> [--name n] [--remap FROM=TO ...]` | `cooling.profiles.import` |
| `service [--mode login\|service] [--boot-profile name]` | `cooling.service.get/set` (empty name clears) |

## Pause (additive, 2026-10-23)

* `lighting.status` -> `{paused}`; `lighting.pause {paused:bool}` ->
  `{paused, released}`. Pausing hands every real device back to hardware mode
  (as with conflicting RGB software); `diagnostics.get` lists `"Paused by you"`
  under `conflicts` meanwhile.
* `cooling.fans.pause {paused:bool}` -> `{paused}`: stops an active interactive
  session and pauses/resumes all background channels in the protected service
  (`freergb-mainboard.exe --background pause|resume|status`, broker pipe
  records `background pause|resume|status`). The service status file reports
  `paused`.

**Fan role instead of per-session confirmation (2026-10-23).** A header bound as
`fan` (fan identification in the setup wizard or on its Cooling card) counts as
confirmed; `confirmed_fan` is only needed for headers without a binding. Pumps
still need the unrestricted unlock plus the pump confirmation, and unassigned
headers are never reduced below 100%.

## Downloadable effect marketplace

The daemon supports the `marketplace.status`, `marketplace.refresh`,
`marketplace.install` and `marketplace.uninstall` methods. `effects.set` also
accepts `{ "type": "marketplace", "id": "aurora", "version": "1.0.0",
"parameters": { "speed": 1, "brightness": 0.8 } }`. This reference is saved in
normal state/profiles and resolved from verified installed packages on restart.
See [the marketplace API and security contract](EFFECT_MARKETPLACE.md) for job
completion, ABI, installation, permissions and error behavior.

## Linux hwmon fan control (additive, 2026-10-24)

`cooling.hwmon.get`, `cooling.hwmon.set` and `cooling.hwmon.release` control
motherboard fan headers through the Linux kernel hwmon drivers. Parameters,
limits and recovery behaviour: [COOLING_LINUX_HWMON.md](COOLING_LINUX_HWMON.md).
The provider `mainboard.hwmon` is appended to `cooling.providers.list`, its
channels to `cooling.channels.list` and its readings to `monitoring.sensors.list`.
