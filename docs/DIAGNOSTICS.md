# Diagnostics and sensor history

The Monitoring tab records at most 600 samples per sensor (normally ten minutes
at the GUI's one-second polling interval), across all tabs while the GUI is open.
At most 128 sensor series are retained. Missing, disconnected, nonfinite or stale
(over 3 s) samples are gaps, never interpolated readings. Devices absent for more
than 60 s are pruned. History is in memory only; it is not a hardware sampler and
does not add USB requests. Cached LINK RPM channel identity is still unconfirmed.

`diagnostics.export` returns report format version 1. The Monitoring tab's
**Save diagnostic report** button writes it locally under the per-user config
`freeRGB/diagnostics` directory (Linux: `freergb/diagnostics`). A timestamped file
is created without overwriting an existing file. No upload or external message.

Report fields: application/platform version, runtime/FPS/effect type, public USB
VID/PID and device kinds, frame counters, anonymous device/plugin/sensor indexes,
sensor units/values/age and fault flags, plus `cooling_channels`: per anonymous
motherboard fan control its support `tier`, `register_readback`, `software_mode`
and `held_full` flags (no names, identities or readings). `diagnostics.get`
lists the same as `cooling_support` with channel names.

Opt-in (`diagnostics.export {include_cooling_hardware:true}`, the second button in
both GUIs): `cooling_hardware` adds the public SMBIOS board manufacturer/product,
BIOS version, the pinned LHM version and per anonymous control its chip name,
tier, register readback, channel-test state and whether it is unlocked. Purpose:
promote LHM-supported chips with evidence. Still no serials, names, readings or
paths; nothing is uploaded. Device IDs, names/serials, capture names,
paths, raw errors and macro/profile contents are omitted. Review before sharing.
Full local `diagnostics.get` and `devices.list` are unchanged; this is a separate
report with deliberately reduced identifying data.
