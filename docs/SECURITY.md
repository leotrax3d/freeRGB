# Security

* Unknown devices are only enumerated, never written to. The daemon opens only
  interfaces whose adapter is at least `Experimental` in the registry.
* Built-in hardware writes only via validated adapters; property writes via allowlist.
* No firmware, flash, driver or SMBus writes without explicit user approval.
* Devices are returned to hardware mode on disconnect and daemon exit.
* `unsafe_code = "forbid"` across the workspace.
* Local API: IPC only (named pipe without remote clients / Unix socket `0600`), no TCP port.
* No personal data from SignalRGB logs (e.g. machine ID) is copied into the project.
* Installing `freergbd` as a system service or autostart is a system change and requires
  the user's approval.
* Conflict detection: while SignalRGB (`SignalRgb.exe`, launcher), Corsair iCUE (`iCUE.exe`) or
  OpenRGB runs, `freergbd` releases all hardware devices (back to hardware mode) and opens none;
  checked at start and on every hot-plug scan. `SignalRgbService.exe` alone is not a conflict.
* Audio and screen capture run only while an audio or screen effect is active and stop when
  another effect is selected. Only derived data leaves the capture thread (8 band levels,
  a 64×36 colour grid); nothing is stored or transmitted.

## Native process plugins

Only explicit `--plugin` manifests can add executable code. Start/stop API calls
reference those IDs only. Plugins run with the user's privileges; process isolation
contains ordinary crashes/hangs, not malicious code. `access` is self-declared.
No sandbox, signature verification or descendant-process containment is claimed.
Plugins must release hardware before acknowledging shutdown and on stdin EOF;
forced termination cannot promise hardware restoration. No hardware process plugin
is validated/shipped. See `PLUGIN_SYSTEM.md` for the protocol and trust boundaries.

`--no-hardware` blocks declared hardware plugins. Configuring one hardware plugin
excludes all built-in adapters; multiple hardware plugins are rejected because the
HID gate is process-local. Conflict detection suspends the hardware plugin too.
Child request/response sizes and request duration are bounded; executable startup
latency depends on the OS. Stderr is discarded, and plugins must not spawn children.

Local API requests are capped at 256 KiB per line; oversized connections close.
Effect parameters and saved state/profile data are validated before application.
