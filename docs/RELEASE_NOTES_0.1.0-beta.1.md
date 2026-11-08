# freeRGB 0.1.0-beta.1

First public beta of freeRGB for Windows (x86_64).

## Highlights

* Modern desktop GUI (Tauri + React) with lighting, effects, devices, cooling
  and settings views; the classic GUI remains available.
* Rust background service (`freergb-daemon`) that keeps lighting and fan
  curves running with the GUI closed, plus tray notifications.
* Device detection through the signed hardware-module catalog; modules are
  installed from the first-run wizard.
* Effect system with layered built-in effects and a sandboxed, verified
  WebAssembly effect marketplace.
* Motherboard fan control (LibreHardwareMonitor/PawnIO) behind a protected
  service, safety acknowledgement, fan identification and bounded channel tests.

## Installer

`freeRGB-0.1.0-beta.1-windows-x86_64-setup.exe` runs elevated once, installs
the app, .NET 4.8 and PawnIO if missing, the bundled LHM runtime and the
protected motherboard service, then starts the GUI as the signed-in user. The
first-run wizard shows the safety notice, detects devices, installs matching
extensions and offers guided fan tests.

## Known issues

* The installer and app are not code-signed yet; SmartScreen will warn.
* Fan control and the wizard's fan identification have not been validated on
  a broad range of real motherboards. Pumps and unconfirmed headers are never
  reduced, but treat fan control as experimental.
* Several hardware modules are implemented from protocol documentation and
  are untested on physical devices (see `docs/DEVICES.md`).
* The marketplace's ratings and issue reports need the Supabase project
  configured (`docs/SUPABASE_SETUP.md`); without it they are hidden.
* Linux builds are source-only for this beta.
* The safety/liability text has not had legal review.
