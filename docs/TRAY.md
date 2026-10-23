# Autostart and tray icon

## Autostart

Setup always registers `freergb-launcher.exe --daemon-only` in
`HKLM\Software\Microsoft\Windows\CurrentVersion\Run`: at every user's sign-in
the freeRGB service (lighting, effects, profiles, interactive fan sessions)
and the tray icon start without the GUI. Saved background fan curves already
run from boot in the protected SYSTEM service, independent of sign-in. There is
no autostart checkbox anymore (owner decision 2026-10-23).

## Tray (`crates/freergb-tray`)

A small separate process (Rust, `tray-icon`), one per session (named mutex
`Local\freeRGB.Tray.v1`). It only uses the daemon's local RPC API, like the GUI;
administrator consent for fan configuration still happens via UAC in the daemon.
It polls every 3 s; actions run on worker threads so a UAC prompt never freezes it.

Menu:

* Status lines: service/lighting state and fan state (running curves, faults,
  paused, locked until the safety notice is accepted, unavailable).
* Open freeRGB / Open freeRGB Classic (left click on the icon opens the GUI).
* Pause lighting (`lighting.pause`): devices return to their own hardware
  lighting, like when conflicting RGB software runs; resume restores the effect.
  Persisted in the daemon state; survives service restarts and reboots.
* Brightness 25/50/75/100 % and lighting profiles.
* Pause fan control (`cooling.fans.pause`): stops an interactive fan session and
  asks the protected service to return every background channel to firmware
  (BIOS) control. No UAC: pausing only reduces control and resuming restarts the
  configuration the user already authorized. Paused until resumed, also across
  service restarts and reboots (`mainboard-control-paused.json`). Unavailable while the safety notice is not accepted.
* Cooling profiles (`cooling.profiles.activate`, asks for UAC).
* Start freeRGB service (when it is not running) / Quit freeRGB
  (`daemon.shutdown`; background fan curves keep running in the protected
  service).

The icon is the brand Ring (`assets/icons/freergb-32.png`), greyed out when
nothing is actively controlled. Uninstall ends the tray process first.
