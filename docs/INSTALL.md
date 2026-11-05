# Installation

The Windows installer can be built locally or through the GitHub Package workflow.
See the README for current prereleases and known issues. For actual hardware use, close SignalRGB/iCUE
first; the daemon also detects common conflicts. Virtual demo mode can run alongside them.

## Windows 10 / 11

New to freeRGB? [USER_GUIDE.md](USER_GUIDE.md) walks through first run, profiles, fan
control safety and troubleshooting (including a stuck motherboard reader and a busy
daemon pipe).

Installer artifact: `target/dist/freeRGB-<version>-windows-x86_64-setup.exe` after packaging.
It installs machine-wide under Program Files with an uninstaller and an initial
UAC prompt. Start Menu entries (including Demo) default on; the desktop shortcut
defaults off. Machine-wide login startup always starts the daemon/tray.
Windows login starts only the background daemon, without opening a GUI. The main
shortcut opens Tauri; the separate **freeRGB Classic** shortcut opens the fallback.
The modern GUI requires the Microsoft Edge WebView2 runtime on Windows.
Shortcut choices are retained on upgrades; deselection removes those shortcuts.
Interactive setup prepares verified motherboard prerequisites and the protected
service. Silent setup skips these components unless the exact `/WITHMAINBOARD`
argument is supplied. Fan control still requires separate safety acknowledgement
and channel identification. See
`MAINBOARD_INSTALLATION.md`. The README logo is
used for Setup, installed executables, shortcuts and the GUI window.
See `UPDATES.md`. Running the installer is a separate user action.

An optional **Andere RGB-Programme** page lists recognized installed SignalRGB,
Razer Synapse/Chroma/Central, Corsair iCUE, OpenRGB and Logitech G HUB entries.
Everything is unchecked. Select products and click the explicit uninstaller button,
then confirm the list to start their registered manufacturer uninstallers. **Next**
alone never uninstalls anything, and silent setup never runs these uninstallers.
Manufacturer dialogs may require administrator approval or a restart. Unsupported
or ambiguous entries can be removed through Windows Settings → Apps. Setup does
not execute per-user HKCU uninstaller commands with its elevated privileges;
those programs and legacy per-user freeRGB installs must be removed through Apps.
Setup does not delete third-party files, drivers or registry leftovers itself. Actual vendor
uninstall acceptance has not been performed.

Upgrade/uninstall requests graceful shutdown only if the daemon's canonical
executable path matches the installed copy. Other checkouts are left running.
Older daemons without this guarded API must be stopped manually before upgrading.
Close the GUI when prompted by Windows/Setup; the tray helper only terminates
the installed executable after checking its process handle and full path. Uninstall preserves `%APPDATA%\freeRGB` (profiles, curves, macros and settings).

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
./scripts/build-desktop.ps1 -OutputDirectory "$env:USERPROFILE/.cargo/bin"
```

Run `freergb-launcher` (or `freergb-launcher --demo` for the virtual demo).
The launcher starts the daemon when needed and opens Tauri. Use `--classic` to open
the fallback or `--daemon-only` to start without any window. Desktop builds also
require Node.js 22+ and npm. No administrator
rights are needed for supported USB HID devices. For backend development the daemon
also accepts `--enable-link-hub` for experimental LINK RGB.

For source installations, autostart is not set up automatically. To start the service at logon, create a shortcut to
`%USERPROFILE%\.cargo\bin\freergb-launcher.exe --daemon-only` in `shell:startup`.

State and profiles: `%APPDATA%\freeRGB\` (`state.json`, `profiles\`).

## Linux

Use stable Rust (the locked dependency graph requires Rust 1.90 or newer;
the current audit used Rust 1.99.1) and Node.js 22 or newer for the desktop UI.

Linux audio, screen and keyboard capture use the same effect/input APIs as Windows.
Fixture tests cover monitor selection/PCM, frame transport and evdev decoding; real
Linux desktop/hardware and package installation acceptance remain open. The Linux
packages below ship the daemon, CLI, capture helper and Classic GUI. Tauri remains
an optional source build.

Build dependencies (Debian/Ubuntu names):

```bash
sudo apt install build-essential pkg-config libudev-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev \
  libpipewire-0.3-dev libspa-0.2-dev libclang-dev libasound2-dev libgbm-dev \
  libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

Build and install:

```bash
git clone https://github.com/leotrax3d/FreeRGB.git
cd FreeRGB
cargo install --path crates/freergb-daemon
cargo install --path crates/freergb-cli
cargo install --path crates/freergb-gui
cargo install --path crates/freergb-screen --bin freergb-screen-capture
cd apps/desktop
npm ci
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol --release --locked
install -m 0755 src-tauri/target/release/freergb-desktop ~/.cargo/bin/freergb-desktop
cd ../..
```

Keep `freergb-screen-capture` beside `freergbd`. Audio capture additionally requires
`pactl` and `parec` (`pulseaudio-utils` on Debian/Ubuntu/Fedora), with PulseAudio or
PipeWire's Pulse compatibility server running in the same user session.

### Distribution packages

Build on the target Linux distribution/architecture; generated ELF dependencies
reflect that build host, so a new distribution build is not promised to work on
older releases. Scripts never install a package, change services, or open hardware.

```bash
# .deb: dpkg-dev and the build dependencies above must already be available.
bash packaging/linux/build.sh deb
# .rpm: use a Fedora build environment with rpmbuild and equivalent development libraries.
bash packaging/linux/build.sh rpm
# AppImage: trusted linuxdeploy + its AppImage output plugin must already be on PATH.
# Set APPIMAGE_EXTRACT_AND_RUN=1 when building without FUSE.
bash packaging/linux/build.sh appimage
```

Artifacts appear in `target/linux-packages/`. `package.sh` can package previously
built binaries; `FREERGB_BIN_DIR`, `FREERGB_PACKAGE_DIR` and `FREERGB_ARCH`
(`x86_64` or `aarch64`, matching the binaries) override its defaults.
`bash packaging/linux/test-stage.sh` checks a synthetic payload without running it.
No package post-install hooks enable services or broad keyboard permissions.
Native packages include `/usr/lib/systemd/user/freergbd.service` with
`ExecStart=/usr/bin/freergbd`, desktop metadata and supported-HID udev rules.
Install/remove `.deb` and `.rpm` through your distribution's package manager.
After installation, reload udev rules and replug supported devices as below.

The AppImage opens Classic by default; `--daemon` and `--cli` select the daemon
or CLI. Start `./freeRGB-<version>-<arch>.AppImage --daemon` in your graphical
session before opening its GUI. It still requires a host audio server,
`pulseaudio-utils`, desktop portal and host device permissions. It does not install
udev rules or a user unit. For AppImage autostart, use an explicit user-unit
`ExecStart=/absolute/path/to/freeRGB.AppImage --daemon` override instead of the
source-install path below.

### Audio and screen capture

Audio-reactive effects capture `<default-playback-sink>.monitor` using `parec`
at 48 kHz stereo, then downmix/analyze in memory. They never select a microphone
or change the default source. A missing monitor/server/tool is an error. After
changing the default output, switch to a non-audio effect and back to open the new sink. Capture
stops when no audio effect is active; server loss clears the published features.

X11 uses the primary monitor (first monitor fallback). Wayland uses xcap's
ScreenCast portal/PipeWire recorder, with one compositor permission chooser per
capture session. Install `xdg-desktop-portal` and the backend matching your desktop,
and run in the graphical login session with its D-Bus/PipeWire environment.
The current xcap monitor enumeration also requires Xwayland/`DISPLAY` on Wayland;
pure Wayland without Xwayland is not supported by this backend. The chooser's
selected output is authoritative. Do not repeatedly retry a denied request.
Initial permission/capture can wait up to 60 seconds in the background, without
blocking daemon API/render work. Missing/denied capture leaves
the effect dark; existing screen diagnostics show waiting/unavailable status.
After correcting dependencies or denying consent, switch to a non-screen effect
and back to retry. Switching away
from a screen effect terminates/reaps the helper, closing the portal connection.
Only downscaled 64 x 36 RGBA frames leave the helper; nothing is saved to disk.

### Linux input permissions

Keyboard-reactive lighting is opt-in through the existing keyboard configuration.
It reads `*-event-kbd` links under `/dev/input/by-id` and `/dev/input/by-path`
without grabbing devices, translating text, persisting events or injecting input.
Key repeats/releases and stale queued presses are discarded. Missing devices are
rescanned and permission failures appear in keyboard status.

Keyboard device access exposes physical keystrokes across applications, including
password entry. For a selected keyboard, an administrator can grant a narrow ACL
or device-specific udev rule. If you explicitly want capture from all local
keyboards, the **optional** example below grants the active local seat access;
it is deliberately not installed as an active rule by packages:

```bash
sudo install -m 0644 packaging/linux/70-freergb-input.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
# Replug the selected keyboard or log out/in; then enable keyboard reactions.
```

Packaged copy: `/usr/share/doc/freergb/70-freergb-input.rules.example`.
Do not run the daemon as root or add yourself to the unrestricted `input` group.
Removing the optional rule and logging out/in revokes the grant. Disabling the
effect closes its input handles. `--no-hardware` never opens input devices.

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

The commands above install the **source-build** unit. Native packages already
provide a unit; use `systemctl --user enable --now freergbd` after installation
instead of copying the source unit. Run only one daemon per user. For capture
from a user service, ensure the graphical session imports `DISPLAY`,
`WAYLAND_DISPLAY` and `XDG_SESSION_TYPE` into the user manager (for example with
`systemctl --user import-environment DISPLAY WAYLAND_DISPLAY XDG_SESSION_TYPE`).
Do not enable lingering for a graphical capture service.

State and profiles: `$XDG_CONFIG_HOME/freergb/` (usually `~/.config/freergb/`).
API socket: `$XDG_RUNTIME_DIR/freergb.sock`.

### Owner acceptance steps

On disposable Debian/Ubuntu and Fedora desktops, build and inspect the generated
native package (`dpkg-deb --info/--contents`, `rpm -qpi/-qpl`), install it with
the package manager, and confirm no service starts automatically. Verify launch,
explicit user-service start/stop, SIGINT cleanup, upgrade/removal and preserved
profiles. Build/extract/run the AppImage and check its GUI, daemon and CLI modes.

On X11 and Wayland with the appropriate portal backend, play known audio and
confirm monitor-only reactivity (including silence/server loss); approve, deny,
cancel and revoke screen sharing, change effects, and check helper/portal cleanup.
Test primary/output selection, colors, capture timeout and compositor restart.
Without input permission, verify the reported error; grant only the intended
keyboard, enable reactions, check physical layout, disconnect/reconnect, disable
capture and confirm handles close. Real typing/LED writes require owner consent;
fixture/staging tests do not establish any of these hardware acceptance results.

## Uninstall

```bash
cargo uninstall freergb-daemon freergb-cli freergb-gui
```

Linux: remove `/etc/udev/rules.d/60-freergb.rules` and the user unit. Windows source
installations: remove your startup shortcut. Installer installations: use Windows
Settings → Apps → freeRGB → Uninstall. Delete the state directory separately only
if you want to drop profiles and settings.


## Optional sensor and GPU components

Nothing here is mandatory. CPU/GPU/storage temperatures on Windows come from the
optional LHM provider below; on Linux from the kernel hwmon drivers (`k10temp`,
`coretemp`, `amdgpu`, `nvme`, `drivetemp`). NVIDIA GPU fan control uses the NVML
library that ships with the NVIDIA driver (no extra install) and needs
administrator/root rights to set speeds. AMD Windows (ADLX) control is not
available. See `COOLING_SOURCES_GPU.md`.

## Optional motherboard backend

Normal installation/startup requires no PawnIO, LHM or additional .NET runtime.
The experimental Windows motherboard reader is opt-in and currently read-only.
Setup includes our helper and an unchecked optional page for verified LHM/PawnIO
and missing .NET 4.8 downloads/installation. It does not bundle upstream installers.
See `MAINBOARD_INSTALLATION.md`; preparing dependencies does not yet solve the
normal-user privilege broker or enable real fan control. Advanced users with a trusted
LHM 0.9.6 runtime, an already-running PawnIO driver and the required rights can use
`freergb-launcher --mainboard-library "C:\path to\LHM"`. The option is forwarded
only when starting a new daemon; it does not reconfigure an already-running daemon.
It is incompatible with `--demo`. The launcher itself never requests automatic elevation; an existing successful
installer opt-in also selects the prepared runtime without editing shortcut targets. See `MAINBOARD_SUPPORT.md` for limitations and outstanding validation.

Experimental driver-free motherboard RGB/ARGB via known ASUS USB interfaces is
documented in `MAINBOARD_RGB.md`; it requires separate explicit activation.
