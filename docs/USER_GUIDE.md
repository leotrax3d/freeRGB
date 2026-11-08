# freeRGB user guide

This guide covers everyday use of freeRGB on Windows. Features that are only
verified with simulations or fixtures say so; the authoritative status is
[STATUS.md](STATUS.md). Technical background lives in the other documents in
this folder and is linked where relevant.

## 1. Install

1. Download the latest `freeRGB-<version>-windows-x86_64-setup.exe` from the
   [GitHub releases](https://github.com/leotrax3d/FreeRGB/releases) (or the
   latest Package build from GitHub Actions). Windows asks for administrator
   approval once because the installer writes to Program Files.
2. Close other RGB programs that control the same hardware first (SignalRGB,
   Razer Synapse, Corsair iCUE, OpenRGB, Logitech G HUB). freeRGB detects common
   conflicts and leaves affected devices alone, but two programs writing to one
   device still cause flicker.
3. Choose the Start Menu and optional shortcuts. The installer also offers
   optional pages for other-RGB-program removal and motherboard prerequisites;
   everything on those pages is unchecked by default and nothing is removed or
   installed unless you select it and confirm.
4. Finish setup. Details, silent install switches and source builds:
   [INSTALL.md](INSTALL.md).

The modern interface needs the Microsoft Edge WebView2 runtime. A fallback
interface, **freeRGB Classic**, has its own shortcut.

Uninstalling keeps `%APPDATA%\freeRGB` (profiles, curves, macros, settings).
Delete that folder yourself to remove your data.

## 2. First run

Start **freeRGB** from the Start Menu. The launcher starts the background
daemon (`freergbd`) if needed and opens the window; the tray icon stays after
the window closes. At Windows login only the daemon and tray start, not a window.

- **Devices** lists what was found, with firmware and connection status. If a
  device is missing, check the [troubleshooting](#7-troubleshooting) section and
  the support lists on the website.
- Use **Demo mode** (`freergb-launcher --demo`, or the Demo shortcut) to try the
  interface with virtual devices. It never touches real hardware and can run
  next to other RGB software.

freeRGB needs no account, sends no analytics or crash reports, and exposes no
network port; the local API uses a Windows named pipe ([LOCAL_API.md](LOCAL_API.md)).
It does contact GitHub when you download effects, modules or updates. The
optional online community features (ratings, reports) and download counting are
off by default and are switched in the Marketplace page ("Online community and
privacy"); see [PRIVACY.md](PRIVACY.md) (draft).

## 3. Lighting

- **Lighting**: pick an effect, adjust colour, speed, direction and scale, and
  set the global brightness. Eleven built-in effects exist, plus layered custom
  effects.
- **Layout**: drag devices onto the shared canvas so an effect flows across the
  whole desk instead of repeating on every device.
- **Audio reactive** and **screen ambience** effects use local capture on
  Windows; audio and screen content never leave your machine.
- Keyboard-reactive lighting (key presses lighting up keys) needs the optional
  keyboard helper; see [KEYBOARD_REACTIVE.md](KEYBOARD_REACTIVE.md).

## 4. Profiles

A profile stores the complete lighting state (effect, parameters, brightness)
under a name. Save one after you are happy with a setup, then switch from the
Profiles page, the tray menu or the local API. Optional application rules can
switch profiles when a chosen program is in the foreground
([APPLICATION_PROFILES.md](APPLICATION_PROFILES.md)); they are off until you
create a rule.

Profiles live in `%APPDATA%\freeRGB\profiles\`. Copy the folder to back it up.

## 5. Macros, plugins and Linux

- **Macros** (available): the Macros page builds sequences of colour, effect,
  brightness, delay, text and key steps. Sending text or keys to other programs
  needs the daemon option `--enable-input-macros` and goes to whichever window
  has focus, so add a delay that lets you switch windows. Global shortcuts are
  opt-in per session. See [MACROS.md](MACROS.md).
- **Recording macros** (available, simulated verification only): choose
  **Record** on a macro page and type into its pad to capture key presses with
  timing, or reviewed text as one step. Review the draft before saving; drafts
  are not kept across restarts. Mouse recording is not enabled. See
  [MACROS.md](MACROS.md#focused-recording).
- **Per-device macro keys** (available, fixture-tested): assign saved macros to
  F6-F10 on supported keyboards. Physical device attribution still needs owner
  acceptance on real hardware ([MACROS.md](MACROS.md)).
- **Effect plugins** (available, owner acceptance pending): install sandboxed
  WASM effects from the marketplace or collections, enable, disable or remove
  them, and hot-reload a local development effect. Official catalogs must carry
  a valid signature. See [EFFECT_PLUGINS.md](EFFECT_PLUGINS.md). Declarative
  device descriptions and downloadable signed hardware modules also exist
  ([PLUGIN_SYSTEM.md](PLUGIN_SYSTEM.md), [HARDWARE_MODULES.md](HARDWARE_MODULES.md)).
- **Motherboard RGB headers** (available for supported USB controllers): ASUS
  Aura, Gigabyte RGB Fusion 2 and ASRock Polychrome headers can be configured
  with LED counts and colour order, with a fallback when a controller is
  not driven. Boards using SMBus are not supported. See
  [MAINBOARD_RGB.md](MAINBOARD_RGB.md).
- **Updates**: the visible updater checks hashes before launching an installer;
  nothing is published or installed automatically. Cross-version upgrade and
  rollback on real systems are still owner acceptance items
  ([UPDATES.md](UPDATES.md)).
- **Linux**: the daemon, CLI, capture helper (audio, screen, evdev keyboard),
  udev rules and a systemd user service exist, and .deb, .rpm and AppImage
  packages can be built with `packaging/linux/build.sh`. Everything is verified
  with fixtures and CI only; real Linux hardware and package installation
  acceptance remain open. See the Linux section of [INSTALL.md](INSTALL.md).

- **Language**: the modern GUI is available in English and German. Switch it
  under **Settings → Language**; the choice applies immediately and is
  remembered. Untranslated text falls back to English. See [I18N.md](I18N.md).

## 6. Fan control basics and safety

Monitoring (temperatures, fan speeds, history, warnings) is read-only. **Fan control writes to your hardware** and is deliberately
gated. Read this before enabling it.

- Fan control is opt-in. Windows motherboard access uses an optional protected
  component and a separate safety acknowledgement; nothing changes fan speeds
  until you complete guided setup.
- The guided setup identifies each channel: it briefly runs a channel at a known
  speed so you can confirm which physical fan it is, pairs it with its
  tachometer, and then proposes a curve. Do this with the case open or the
  fans visible, and one channel at a time.
- Always keep a safety margin. Set a **minimum output** that keeps every fan
  spinning, use a sensor that really reflects the heat source, and keep the
  **critical-temperature fallback** enabled: if the sensor goes missing, goes
  stale or reaches the critical limit, freeRGB drives the fans to 100%.
- Never put a pump or a single case fan that cools a hot component on a curve
  that can reach zero.
- Most chips are supported through simulation and published reference
  behaviour only; very few have been confirmed on real hardware. The coverage
  tables on the website show the tier of each chip. If your chip is read-only,
  freeRGB will not offer control.
- If anything looks wrong (fans stop, temperatures climb), use the tray
  **stop-all / 100%** action, or close the application and use your BIOS fan
  profile. Background details: [COOLING.md](COOLING.md),
  [COOLING_SAFETY.md](COOLING_SAFETY.md), [FAN_COVERAGE.md](FAN_COVERAGE.md).

## 7. Troubleshooting

### A device is not listed or does not light up
Close SignalRGB, iCUE, Synapse, OpenRGB and G HUB (also from the tray), then
restart freeRGB. Check **Devices** for a conflict message. Replug USB devices
and try another USB port without a hub.

### "Motherboard reader is starting"
The optional motherboard reader (temperatures and fan speeds) is a separate
helper process that needs a few seconds to load its library and driver, so the
status shows *starting* first. Wait up to a minute.

- If it stays in *starting*, open **Settings → Diagnostics** and look at the
  reader state: *missing library* or *missing driver* means the optional
  prerequisites are not installed (rerun setup and tick the motherboard
  components); *permission* means the helper lacks the rights it needs;
  *unsupported hardware* means your board cannot be read.
- Restart freeRGB from the tray (**Quit**, then start it again) to restart the
  helper. Values older than five seconds are shown as stale, not as current.
- Details: [MAINBOARD_SUPPORT.md](MAINBOARD_SUPPORT.md),
  [MAINBOARD_INSTALLATION.md](MAINBOARD_INSTALLATION.md).

### The window says it cannot connect, or the daemon will not start ("pipe busy")
The daemon owns exactly one named pipe, `\\.\pipe\freergb`. A second daemon
cannot start while the first one holds it, and the window cannot talk to a
daemon that is hung. Typical causes: a daemon from another copy of freeRGB
(for example a source build or an older install) is still running, or the
previous daemon did not exit after a crash.

1. Quit freeRGB from the tray icon and wait about 10 seconds.
2. Open Task Manager → Details and look for `freergbd.exe`. If one remains,
   check **Open file location** to see which copy it is. End that task only if
   you do not need it.
3. Start freeRGB again from the Start Menu. The launcher starts a fresh daemon.
4. If it persists, sign out and in again, then export diagnostics
   (**Settings → Diagnostics → Export**; secrets are redacted) and attach the
   file to a [bug report](https://github.com/leotrax3d/FreeRGB/issues).

Do not delete the pipe or edit the registry; the pipe disappears when its
owning process ends.

### Lighting stays on the last colour after closing
Devices keep their last state until the daemon hands them back to hardware
lighting, which happens on a normal quit. After a crash, restart freeRGB and
quit it normally.

### Reset
Quit freeRGB, rename `%APPDATA%\freeRGB` and start again for a clean state; copy
`profiles\` back to keep your profiles.

## Where to look next

[INSTALL.md](INSTALL.md) · [STATUS.md](STATUS.md) · [DEVICES.md](DEVICES.md) ·
[OPENRGB_COMPATIBILITY.md](OPENRGB_COMPATIBILITY.md) · [FAN_COVERAGE.md](FAN_COVERAGE.md) ·
[SECURITY.md](SECURITY.md)
