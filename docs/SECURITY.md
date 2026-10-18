# Security

* Unknown devices are only enumerated, never written to. The daemon opens only
  interfaces whose adapter is at least `Experimental` in the registry.
* Writes only via validated adapters; property writes via allowlist.
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
