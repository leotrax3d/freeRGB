# Installation

The Windows installer can be built locally or through the GitHub Package workflow.
There is not yet a published release. For actual hardware use, close SignalRGB/iCUE
first; the daemon also detects common conflicts. Virtual demo mode can run alongside them.

## Windows 10 / 11

Installer artifact: `target/dist/freeRGB-0.1.0-windows-x86_64-setup.exe` after packaging.
It installs per user with start-menu entries, a Demo shortcut and an uninstaller.
Desktop shortcut and login startup are optional; no driver/system service is installed.
See `UPDATES.md`. Running the installer is a separate user action.

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

Run `freergbd` (add `--enable-link-hub` for iCUE LINK fans/AIO), then `freergb-gui` or
`freergb ctl …`. No administrator rights are needed for USB HID devices.

Autostart is not set up automatically. To start the service at logon, create a shortcut to
`%USERPROFILE%\.cargo\bin\freergbd.exe` in `shell:startup`.

State and profiles: `%APPDATA%\freeRGB\` (`state.json`, `profiles\`).

## Linux

Status: Linux hardware is untested. An older CI run failed because PipeWire development
libraries were missing; the dependency list has been corrected and a new result is pending.

Build dependencies (Debian/Ubuntu names):

```bash
sudo apt install build-essential pkg-config libudev-dev \
  libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev \
  libpipewire-0.3-dev libspa-0.2-dev libclang-dev libasound2-dev
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

Linux: remove `/etc/udev/rules.d/60-freergb.rules` and the user unit. Windows: remove the
startup shortcut. Delete the state directory if you want to drop profiles.
