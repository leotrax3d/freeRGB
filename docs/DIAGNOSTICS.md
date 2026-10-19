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
sensor units/values/age and fault flags. Device IDs, names/serials, capture names,
paths, raw errors and macro/profile contents are omitted. Review before sharing.
Full local `diagnostics.get` and `devices.list` are unchanged; this is a separate
report with deliberately reduced identifying data.
