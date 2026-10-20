# Optional motherboard components in Windows Setup

The installer has a dedicated, **unchecked** opt-in page. It describes downloads,
administrator prompts, possible reboot, experimental discovery/readings and the
fact that real fan control is not yet released. It explicitly includes our protected
LocalSystem sensor service with automatic service startup. Silent/default installation never
downloads or installs prerequisites. Selecting the option runs the preparation
after the core application is installed and its old daemon is stopped.

The embedded PowerShell bootstrap requires explicit `-ConsentToInstall`. It runs
as the installing user and elevates signed upstream system installers (and
`sc.exe start PawnIO` for an existing stopped driver), then our one-shot broker
maintenance executable, through Windows UAC. Our own distribution signing remains
pending. It does
not elevate the entire GUI/daemon or grant general users access to PawnIO.

Pinned components in `packaging/windows/mainboard-packages.json`, version 1:

| Component | Source | Behavior |
|---|---|---|
| LibreHardwareMonitor 0.9.6 | Official GitHub release | Download unmodified archive; deploy flat runtime DLLs, excluding reference GUI/dependencies specific to it |
| PawnIO 2.2.0 | Official GitHub release | If the driver is missing, launch verified official installer with its own interactive/license dialogs; use signed standard edition |
| .NET Framework 4.8 | Official Microsoft installer | Only if registry Release < 528040; passive installation without forced restart |

Downloads have fixed SHA-256 and byte size; executable installers additionally
require valid Authenticode and the pinned expected publisher. Hash/signature are
checked again before execution. Redirects must finish over HTTPS; downloads are
size-limited, have network timeouts and a 10-minute streaming limit. Unexpected
new upstream bytes fail closed, requiring a reviewed manifest update. The official
PawnIO/NET installers are downloaded at installation time, not redistributed inside
our Setup. No unofficial/unsigned driver or undocumented silent PawnIO flags are used.

Existing .NET/driver installations are reused. A .NET reboot result stops further
preparation and asks the user to reboot/rerun Setup. Failed downloads, cancelled
UAC/manufacturer dialogs or non-success exit codes leave the core app usable and
show an explicit error. Exit codes 3010/1641 are restart-required. PawnIO service
presence alone is not a verified compatibility or signed-driver acceptance result;
driver-backed operation still needs validation.

After prerequisites are ready, Setup captures the original user's SID and the
maintenance image hash **before** UAC. Maintenance verifies/copies its own image,
the build-bound helper and pinned upstream runtime/notices from immutable held
source streams. The service image and DLLs live only in protected
`%ProgramFiles%/freeRGB/MainboardBroker`, never in the writable per-user app/cache.
It registers `freeRGBMainboard` as an automatic own-process LocalSystem service,
with the installing user's SID controlling its read-only pipe access. Only the
fixed protected child loads LHM. GUI and daemon stay unelevated.

Updates selected through the same opt-in stage the new protected files, stop/wait
for the owned service process, retain old files and start the replacement. Failure
restores the old files and prior running state. Foreign names, image paths or
owners fail closed. A failed rollback retains its protected backup and reports
administrator repair; power-loss recovery between renames is not yet automatic.

LHM's MPL license, upstream third-party notices, embedded PawnIO.Modules LGPL-2.1
license and source provenance are installed beside its DLLs. Runtime code is
unmodified and replaceable; source links include the pinned LHM tree and embedded
module release. This is not a claim that vendor-installer interaction or every
runtime dependency path has been tested.

## Installed state (not GUI settings)

Files below live next to `freergb-launcher.exe` in the installation directory,
not in `%APPDATA%/freeRGB`; no shared GUI configuration formats are changed:

* `mainboard-packages.json`: embedded version-1 download manifest (URLs, hashes,
  sizes and expected executable publishers). Never supplied through the local API.
* `providers/lhm-0.9.6/`: optional upstream runtime, license texts and `SOURCES.txt`.
* `mainboard-provider.json`: written only after successful preparation:
  `{"version":1,"enabled":true,"library":"providers/lhm-0.9.6"}`.
  The launcher bounds reads to 4096 bytes, accepts version 1/explicit true/fixed
  relative path only, and requires the DLL to exist. Unknown/malformed state is
  ignored. An explicit launcher library option overrides it; demo ignores it.
* `mainboard-install-result.json`: version 1; `status` (`prepared`,
  `restart_required`, `failed`), `restart_required` bool, and a human-readable
  `message`. Failure/reboot details are preserved locally. This file is diagnostic,
  not authorization to execute or load arbitrary paths.

Normal shortcuts/autostart still target only the launcher. Closing either GUI never
stops the daemon. Upgrades without new opt-in preserve previously prepared runtime
state; they do not rerun prerequisite installation. Uninstall removes our runtime
and state but **does not uninstall shared PawnIO or .NET** needed by other software.
When an owned broker is present, interactive uninstall removes it through explicit
elevated maintenance before deleting the per-user maintenance copy. Cancellation or
failure aborts uninstall before application files are removed, preserving a retry
path. Silent uninstall never requests UAC; an installed broker makes it fail with
an instruction to use interactive removal. The protected parent retains a small
maintenance lock/diagnostic, not an active service or library. Upgrades without
new mainboard opt-in leave the existing service build unchanged.

Protected installed state (`MAINBOARD_BROKER.md`): `installation.json` version 1
inside the broker tree (`owner_sid`, `protocol_version`, `helper_sha256`); owner SID
also lives in its protected HKLM service Parameters key. Parent
`mainboard-maintenance-result.json` version 1 contains `action` (`install`/`remove`),
`status` (`success`/`failed`), `message`, `updated_at_utc`. These are installation
diagnostics, never GUI configuration or permission to execute a supplied path.

**Remaining hardware work:** the broker bridges normal-user requests to the
privileged reader. Dependency installation or a running service does not establish
that a particular motherboard is supported or its fan/source labels are correct.
Real fan writes, calibration and independent failsafe remain separate gated work.
Motherboard RGB/ARGB HID access can work without these prerequisites; see
`MAINBOARD_RGB.md` for the initial experimental backend.

## Verification

`scripts/test-mainboard-prerequisites.ps1` checks detection boundaries, manifests,
tamper rejection and rejection of unsigned payloads even with a matching hash.
`-VerifyDownloads` also downloads official packages, verifies signatures and stages
runtime/notices **without executing any installer or loading any driver**.
The GitHub Package workflow includes this check. The installer lifecycle test checks
that default/silent installation deploys bootstrap files but does not create opt-in
state or register a service. Disposable-runner SCM acceptance uses compile-time
synthetic readings to exercise protected install, a real standard-user client,
failed-start rollback, update, tamper/owner rejection and removal. Real opt-in
wizard/UAC/driver installation and reboot acceptance still need
an isolated Windows machine; none were performed on the developer's PC.
