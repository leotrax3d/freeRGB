# Installation

The Windows installer can be built locally or through the GitHub Package workflow.
There is not yet a published release. For actual hardware use, close SignalRGB/iCUE
first; the daemon also detects common conflicts. Virtual demo mode can run alongside them.

## Windows 10 / 11

Installer artifact: `target/dist/freeRGB-0.1.0-windows-x86_64-setup.exe` after packaging.
It installs per user with an uninstaller. Setup asks independently about Start Menu
entries (including Demo), a desktop shortcut and login startup. Start Menu and
**autostart default on**; desktop defaults off. All options are editable, with previous
choices retained on upgrades. Deselecting an option removes freeRGB's corresponding
shortcut/startup entry. No driver/system service is installed. The README logo is
used for Setup, installed executables, shortcuts and the GUI window.
See `UPDATES.md`. Running the installer is a separate user action.

An optional **Andere RGB-Programme** page lists recognized installed SignalRGB,
Razer Synapse/Chroma/Central, Corsair iCUE, OpenRGB and Logitech G HUB entries.
Everything is unchecked. Select products and click the explicit uninstaller button,
then confirm the list to start their registered manufacturer uninstallers. **Next**
alone never uninstalls anything, and silent setup never runs these uninstallers.
Manufacturer dialogs may require administrator approval or a restart. Unsupported
or ambiguous entries can be removed through Windows Settings → Apps. Setup does
not delete third-party files, drivers or registry leftovers itself. Actual vendor
uninstall acceptance has not been performed.

Upgrade/uninstall requests graceful shutdown only if the daemon's canonical
executable path matches the installed copy. Other checkouts are left running.
Older daemons without this guarded API must be stopped manually before upgrading.
Close the GUI when prompted by Windows/Setup; no forced process termination is
added. Uninstall preserves `%APPDATA%\freeRGB` (profiles, curves, macros and settings).

`scripts/test-installer.ps1 -ModelOnly -Compiler PATH` checks product recognition
and uninstall command parsing without installing anything. The Package workflow
also runs an install/reinstall/uninstall lifecycle on an ephemeral Windows runner,
including task defaults/removal, graceful scoped shutdown and preserved profile
data. Personal machines are explicitly refused for that lifecycle test. Logs are
uploaded as a separate artifact. A passing runner result is required before claiming
full installer lifecycle acceptance. This is same-version upgrade-path testing;
cross-version update, signing and vendor-uninstaller interaction remain separate.

Source build:

Requirements: [Rust](https://rustup.rs/) (stable, MSVC toolchain) and the Visual Studio
Build Tools with the "Desktop development with C++" workload.

```powershell
git clone https://github.com/leotrax3d/FreeRGB.git
cd FreeRGB
cargo install --path crates/freergb-daemon   # freergbd
cargo install --path crates/freergb-cli      # freergb
cargo install --path crates/freergb-gui      # freergb-gui
```

Run `freergb-launcher` (or `freergb-launcher --demo` for the virtual demo).
The launcher starts the daemon when needed and opens the GUI. No administrator
rights are needed for supported USB HID devices. For backend development the daemon
also accepts `--enable-link-hub` for experimental LINK RGB.

For source installations, autostart is not set up automatically. To start the service at logon, create a shortcut to
`%USERPROFILE%\.cargo\bin\freergb-launcher.exe` in `shell:startup`.

State and profiles: `%APPDATA%\freeRGB\` (`state.json`, `profiles\`).

## Linux

Status: Linux hardware is untested. An older CI run failed because PipeWire development
libraries were missing; the dependency list has been corrected and a new result is pending.

Build dependencies (Debian/Ubuntu names):

```bash
sudo apt install build-essential pkg-config libudev-dev \
  libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev \
  libpipewire-0.3-dev libspa-0.2-dev libclang-dev libasound2-dev libgbm-dev
```

Build and install:

```bash
git clone https://github.com/leotrax3d/FreeRGB.git
cd FreeRGB
cargo install --path crates/freergb-daemon
cargo install --path crates/freergb-cli
cargo install --path crates/freergb-gui
```

### Device permissions (udev)

The daemon runs as your user and needs read/write access to the devices' `hidraw` nodes.
[`packaging/linux/60-freergb.rules`](../packaging/linux/60-freergb.rules) grants that to the
logged-in user for supported devices only:

```bash
sudo install -m 0644 packaging/linux/60-freergb.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Check it worked (re-plug the device if needed):

```bash
freergb discover        # supported devices listed with their adapter
freergb bragi probe     # read-only; prints firmware and mode of a K70 CORE
```

### Run as a user service (systemd)

```bash
install -Dm0644 packaging/linux/freergbd.service ~/.config/systemd/user/freergbd.service
systemctl --user daemon-reload
systemctl --user enable --now freergbd
journalctl --user -u freergbd -f      # logs
```

The unit stops the service with SIGINT, which hands every device back to its hardware
lighting before exit.

State and profiles: `$XDG_CONFIG_HOME/freergb/` (usually `~/.config/freergb/`).
API socket: `$XDG_RUNTIME_DIR/freergb.sock`.

## Uninstall

```bash
cargo uninstall freergb-daemon freergb-cli freergb-gui
```

Linux: remove `/etc/udev/rules.d/60-freergb.rules` and the user unit. Windows source
installations: remove your startup shortcut. Installer installations: use Windows
Settings → Apps → freeRGB → Uninstall. Delete the state directory separately only
if you want to drop profiles and settings.


## Optional motherboard backend

Normal installation/startup requires no PawnIO, LHM or additional .NET runtime.
The experimental Windows motherboard reader is opt-in and currently read-only.
Setup includes our helper executable, built using the Windows .NET Framework 4.8
compiler; it does not bundle or install LHM/PawnIO. Advanced users with a trusted
LHM 0.9.6 runtime, an already-running PawnIO driver and the required rights can use
`freergb-launcher --mainboard-library "C:\path to\LHM"`. The option is forwarded
only when starting a new daemon; it does not reconfigure an already-running daemon.
It is incompatible with `--demo`. No automatic elevation or new autostart setting
is added. See `MAINBOARD_SUPPORT.md` for limitations and outstanding validation.
