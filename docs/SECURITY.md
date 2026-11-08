# Security

* Unknown devices are only enumerated, never written to. The daemon opens only
  interfaces whose adapter is at least `Experimental` in the registry.
* Built-in hardware writes only via validated adapters; property writes via allowlist.
* No firmware, flash, driver or SMBus writes without explicit user approval.
* Hardware-mode restoration is attempted on disconnect and orderly daemon exit.
  Cooling recovery depends on the backend and can fail; verified Windows
  motherboard emergencies retain ownership. Crash-journal recovery runs on the
  next start, not while the daemon is stopped. See `COOLING_SAFETY.md` for
  backend-specific claims and limits.
* Most crates forbid unsafe code; the updater permits a documented Windows shell
  call for explicit elevation. This is not a workspace-wide unsafe-code prohibition.
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

## Read-only HID identity extension

Signed HID module manifests may explicitly permit bounded serial/product string
reads and feature report reads during the versioned pre-initialization probe.
The broker denies output/feature writes and input reads in that phase, validates
report IDs and maximum lengths, and denies string reads in all other phases.
Probe strings are transient and are not added to diagnostics or setup state;
modules must not echo them in errors/results. See HARDWARE_MODULES.md for the
wire contract, package compatibility and trusted-native-process limitation.

## Updates and native trust

The updater requires an offline-verifiable Ed25519 signature under a public key
pinned in the binary, binding version, Windows target, size and SHA-256 digest.
Online release metadata cannot supply or replace that key. The actual installer
is hashed and its signature checked again before requesting elevation. No release
key is provisioned yet, so installation fails closed; tests use generated fixture
keys only. See `UPDATES.md` for the signature format and release prerequisites.

Native hardware/effect modules execute trusted code with their host process's
OS privileges. Signatures and broker validation authorize code and constrain
cooperative protocol requests; they do not confine arbitrary native instructions,
file/network access, direct OS calls or child processes. A separate process can
contain ordinary crashes, but no OS-enforced sandbox for hostile native modules
is implemented or claimed. User consent and signature verification do not prove
that signed code is harmless.
