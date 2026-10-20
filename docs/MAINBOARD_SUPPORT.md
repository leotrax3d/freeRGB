# Motherboard-connected fans and RGB

The product target covers users' supported motherboards, not just the developer's
PC. Fan power (3-pin DC / 4-pin PWM) and lighting (5 V addressable / 12 V analog RGB)
are separate electrical interfaces and require separate providers and setup.
An ordinary analog fan/LED strip does not normally identify its model; controller
and header discovery must not be presented as automatic fan-model recognition.
Splitters/hubs may expose only one tachometer for several attached fans.

## Dependency policy

Prefer existing OS interfaces and supported HID protocols. No external driver is
required to start freeRGB or use its existing RGB adapters. A general Windows
SuperIO backend cannot promise fan control through standard user-mode APIs on
every motherboard. A self-written kernel driver would still require installation,
signing, maintenance and hardware-specific validation; it is not a dependency-free
shortcut. Linux hwmon is a separate future provider using existing kernel drivers.

The current Windows LHM provider is **optional and experimental**, off by default.
Normal/default/silent installation does not install PawnIO. An unchecked Setup
page now offers verified official prerequisite downloads and installation after
explicit opt-in, including .NET 4.8 when absent. It includes our small helper and
bootstrap scripts, not bundled upstream driver installers. See
`MAINBOARD_INSTALLATION.md` for consent, hashes, signatures, reboot handling,
installed-state formats and the remaining restricted privilege-broker work.

## Implemented Windows read-only provider

`freergbd --mainboard-library DIR` explicitly trusts a local LibreHardwareMonitor
0.9.6 runtime directory with its dependencies. The daemon launches the sibling
`freergb-mainboard.exe`, built by `scripts/build-mainboard-helper.ps1` using the
Windows .NET Framework compiler. The helper requires installed .NET Framework 4.8.
Use `freergb-launcher --mainboard-library DIR` for the normal GUI entry point;
it forwards the option only when starting a new daemon. It does not reconfigure an
already-running daemon and rejects combination with `--demo`. This is currently
an advanced backend option. Successful installer preparation also writes versioned
installation state that the launcher reads; normal installation remains disabled.
No shared GUI config-root file is added.
The option is rejected with `--no-hardware` and on non-Windows systems.

The helper first tries the fixed protected broker endpoint, verifies the server's
image/protection, and retains measurement age when relaying cached readings. The
broker is currently a tested service candidate, not installed by Setup; see
`MAINBOARD_BROKER.md` for its boundary and outstanding deployment/SCM acceptance.
With no broker, the direct reader checks for the library, an already-running PawnIO
driver and an elevated token before opening LHM. It enables **only** motherboard/SuperIO groups. CPU, GPU,
storage, memory and USB-controller groups remain off. It uses default memory-only
LHM settings, never imports another program's saved control modes and never calls
fan setters. It traverses subhardware and reads temperature, fan and control
sensor values; a reported control object is only a capability candidate.

The daemon polls in a dedicated worker, with a separate native helper process.
Replies are UTF-8 newline JSON, protocol version 1, capped at 256 KiB, 64 hardware
nodes and 1024 sensors. First reply deadline is 10 seconds; subsequent replies
have 3 seconds, with a 1-second delay between requests. No native I/O runs in API,
render or GUI code. Timeout, malformed data or helper exit clears the cache and
faults the provider; no automatic restart loop. Restart the daemon after resolving
a prerequisite or provider fault. Stop cancels even an in-flight native request (25 ms check interval).
Normal stop closes helper stdin, allows one
second for process exit, then terminates a stuck helper. The helper deliberately
does not call stock `Computer.Close()`: `SuperIOHardware.Close()` requests
`SetControl(index,null)`, including for a reader. Because this helper never acquires
software fan control, Windows process exit reclaims handles/memory without invoking
that library cleanup/reset path. This lifecycle must be redesigned before adding
any control writes; native register-discovery side effects still require hardware
acceptance.

The API exposes explicit disabled/starting/missing-library/missing-driver/permission/
unsupported-hardware/fault/stale states. Cached values expire after 5 seconds;
custom Celsius graphs apply their stricter 3-second rule. Timestamps conservatively
include request latency. LHM does not expose per-sensor hardware acquisition
timestamps, so freshness describes a successful provider read, not independent
proof that firmware refreshed a register. Physical labels/source identity still
need confirmation. Missing channels do not establish that no fan is connected.

`cooling.channels.list` keeps RPM and percentage sensors separate: matching list
indices do **not** establish pairing. `physical_mapping` is `unconfirmed`,
`electrical_mode` is `unknown`, and `writable` is always false. No pump classification,
calibration, PWM/DC switching, fan-stop or live curves are enabled. Mainboard
RGB/ARGB is not provided by LHM; the initial experimental ASUS HID adapter is
documented in `MAINBOARD_RGB.md`.

Both GUIs use `cooling.providers.list`, `cooling.channels.list` and
`monitoring.sensors.list`; see `LOCAL_API.md`. Motherboard temperatures also join
the daemon's `cooling.sensors.sources` graph inputs. Classic displays provider
states, channels and readings; it contains no hardware or provider logic.

## Distribution and evidence

Reference source: [LHM 0.9.6](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/v0.9.6),
[Computer groups](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/v0.9.6/LibreHardwareMonitorLib/Hardware/Computer.cs),
[SuperIO controls](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/v0.9.6/LibreHardwareMonitorLib/Hardware/Motherboard/SuperIOHardware.cs).
LHM is MPL-2.0, with additional embedded LGPL-2.1 PawnIO.Modules and third-party
dependencies. Redistribution needs corresponding notices and source availability,
not an MPL notice alone. Exact minimal runtime closure and all dependency licenses
still need verification before bundling. Standalone PawnIO Setup redistribution
terms are not established; our opt-in bootstrap downloads the unmodified official
installer directly instead of bundling it. Its own interactive dialogs remain.

Validated: pure snapshot bounds/duplicate/range checks, disabled hardware gate,
channel separation, no writable capability, expiration and helper compilation/
serialization without hardware. A separate process acceptance test covers the
real helper's missing-runtime response plus synthetic ready, malformed, oversized,
early-exit and hanging replies, cancellation of an active request, bounded cleanup
and paths with spaces. Real LHM runtime/driver-backed discovery on
multiple motherboard families and long-running fault behavior are **not** validated.
No driver was installed or loaded for these tests.

Native alternatives: [Windows Win32_Fan](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-fan),
[Linux hwmon ABI](https://docs.kernel.org/hwmon/sysfs-interface.html).
Presence of a WMI class does not guarantee a usable provider or fan-control method.
