# Motherboard channel assignments

Classic's Cooling page has a **Motherboard channel assignments** editor. Choose
an advertised control, enter a header name such as `CPU_FAN`, select fan/pump,
optionally select its RPM sensor and annotate the reported PWM/DC mode. Click
**Save channel assignments** to persist the complete set. No sensor is paired by
list position, name similarity or matching numeric suffix. An output may have
no tachometer; a missing RPM wire does not prove that a fan is absent.

This is an annotation stage preceding calibration. A user assignment is not
hardware verification and does not authorize control, switch electrical modes,
start a curve or set pump/fan speed. Existing `cooling.channels.list` continues
to report `physical_mapping:unconfirmed`, `electrical_mode:unknown` and
`writable:false`. Reported mode in the new file describes the user's information,
not an OS/firmware measurement. RGB/ARGB is an independent connection/backend.

## File

The daemon owns `cooling-channel-bindings.json` beside `state.json`, normally
`%APPDATA%/freeRGB` or `$XDG_CONFIG_HOME/freergb`. Both GUIs use RPC; no GUI reads
or writes this file. Format version 1:

```json
{
  "version": 1,
  "bindings": [
    {
      "provider": "mainboard.lhm",
      "device_id": "mainboard.lhm:/example/superio",
      "control_id": "[\"/example/superio\",\"/example/control/7\"]",
      "tachometer_id": "[\"/example/superio\",\"/example/fan/2\"]",
      "header": "CPU_FAN",
      "role": "fan",
      "electrical_mode": "unknown"
    }
  ]
}
```

IDs are opaque: copy them from `cooling.channels.list`. The example is not a
real device identity. `tachometer_id` may be null/omitted. Electrical mode defaults
to `unknown` if omitted; other required fields have no implicit defaults. Unknown
fields are ignored for additive clients. Limit: 128 assignments and 256 KiB
serialized/read bytes. Provider currently must be `mainboard.lhm`; device IDs
are nonblank, <=1024 UTF-8 bytes; channel IDs <=4096 bytes; header names are
trimmed/nonempty, <=64 bytes. Text fields reject control characters. Roles:
`fan`/`pump`; modes: `unknown`/`pwm`/`dc`.

Each control and non-null tachometer may occur once per provider. Header names
are unique per device after Unicode lowercase conversion. Fan and control must
belong to the specified same device, and current cached kinds/capabilities must
match when available. No automatic migration to another device or channel occurs
when hardware disappears or enumeration changes. Offline imports/edits are
allowed and remain unresolved until the referenced inventory returns.

Startup reads are bounded; invalid/oversized files leave an empty configuration
and an error visible through RPC. Saves validate before replacing a same-directory
temporary file; failures preserve the active configuration. There is no backup or
power-loss recovery guarantee. `--no-state` stores annotations in memory only.

## Resolution and API

`cooling.bindings.get` returns `config`, `assignments`, `error` and
`persistence_enabled`. Each assignment has `control_id`, `state`,
`hardware_verified:false` and `writable:false`. States are:

- `unavailable`: a referenced control/tachometer is absent.
- `identity_mismatch`: an available referenced channel belongs to another device.
- `incompatible_channel`: wrong fan/control kind or no advertised control object.
- `stale`: all identities match but a reading is missing or older than 3000 ms.
- `user_assigned`: identities/kinds and fresh readings match; physical pairing
  remains unverified.

Known identity/kind mismatches take precedence over absent channels. Save rejects
those mismatches but allows unavailable/stale references. Saved annotations are
not silently dropped when readings expire, service stops or hardware disconnects.
Get resolves current cache contents on demand without native I/O.

`cooling.bindings.save {config}` replaces the full set and returns `true`. Invalid
format/bounds/current identity/kind/capability conflicts return `-32602`; file I/O
errors return `-32000`. Prior state survives either error. All changes are additive
at API version 1. Pure validation/resolution is in `freergb-control::channels`,
persistence/identity checks in the daemon, and Classic is only an RPC editor.
No slider setter is added to `is_setter()`; save is an explicit button action.

Hardware calibration, stall detection, safe write/restore behavior and an
independent watchdog remain required before any live fan-control feature.
