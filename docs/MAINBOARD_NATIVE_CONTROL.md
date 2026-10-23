# Native control adapter candidate

`providers/mainboard/NativeControl.cs` adds an owned adapter for the pinned LHM
0.9.6 `Control` object. It is compiled into the motherboard helper but is **not
called by the ordinary snapshot path**. The snapshot pipe stays read-only;
the separate protected live-session API requires fan confirmation and UAC.
There is also an explicit administrator-only fixed100 diagnostic through
`scripts/probe-mainboard-100.ps1 -ConsentToProbe`: it submits a protected one-use
request and SCM command128, verifies registers and returns the original mode.
Its `-CrashWorker` option tests recovery only during that100-percent operation.
Installer opt-in does not enable normal fan curves or general fan setters.
See `COOLING_RUNTIME.md` for real acceptance and remaining production integration.

## Protected construction and ownership

Production construction is private except for `CreateProtected`. That factory
first checks LocalSystem identity, the fixed protected helper image and protected
runtime tree through `BrokerService.CheckInstallation(true)`. The supplied object
must originate from the protected `LibreHardwareMonitorLib.dll`, have the exact
LHM control type/assembly version 0.9.6.0 and the expected internal metadata and
enum values. Required native range must include the selected floor and 100%.

This is an additional gate, not a replacement for a future authenticated control
transport. Metadata checks alone do not authenticate an arbitrary supplied DLL;
the protected deployment and its verified package remain the trust boundary.

Acquisition refuses a cached Software/unknown mode, reserves the control object
and initially requests 100%. Only this adapter instance can change or release
its reservation. Multiple adapters for the same object share a reservation;
different objects/processes referring to the same physical channel are **not**
serialized by this mechanism. It cannot detect foreign vendor software or BIOS
changes from LHM's cached mode alone. Future activation requires an exclusive
protected worker/session and physical identity/ownership validation.

Fans accept only finite 40..100% duties; pumps only 100%. Invalid/unowned writes
are rejected before touching the mode/value or event callbacks. Losing the owned
Software mode prevents further duties until explicit recovery. These conservative
candidate bounds are not a measured operating range for arbitrary fans.

## Preventing intermediate zero duty and skipped recovery

The pinned [LHM Control implementation](https://raw.githubusercontent.com/LibreHardwareMonitor/LibreHardwareMonitor/v0.9.6/LibreHardwareMonitorLib/Hardware/Control.cs)
changes control mode before updating its stored software value. Its
[SuperIO event wiring](https://raw.githubusercontent.com/LibreHardwareMonitor/LibreHardwareMonitor/v0.9.6/LibreHardwareMonitorLib/Hardware/Motherboard/SuperIOHardware.cs)
writes the currently stored value when Software mode is entered. A fresh cached
zero would therefore produce an intermediate zero command if the public setter
were used directly. A no-hardware test reproduces this with the real shipped
Control class and synthetic callbacks.

The adapter prepares the desired software value while the **memory-only** cached
mode is Undefined, then enters Software once. Repeating an identical duty also
re-enters the mode so the native callback actually runs again. The implementation
uses version-pinned private metadata; any LHM upgrade requires re-auditing and
rerunning these tests, not simply widening the version check.

LHM can also update its cached mode before a native callback throws. A subsequent
plain `SetDefault` could then do nothing because Default is already cached. The
adapter resets only that cached field before each owned restoration attempt so
the callback is really retried. It never calls `Computer.Close()` or resets all
controls. A successful release invokes the selected control's `SetDefault`, then
releases its reservation. Unowned/repeated releases do nothing.

Acquisition/write failures attempt selected-channel restoration. If restoration
also fails, both errors are reported and ownership remains reserved for retry.
A successful callback return is **not physical register/RPM readback**. Firmware
restoration is requested through the backend; its actual effect remains unproven.
Native I/O may block, and the in-process reservation lock cannot rescue a hung
driver/callback. Production activation remains blocked until a separately usable
recovery path, lease/sensor watchdog and physical readback are integrated.

## Acceptance without opening hardware

Run `scripts/test-mainboard-control.ps1`. It downloads/verifies the pinned official
archive and notices, stages only runtime libraries, and constructs actual LHM
Control objects with interface stubs for settings/sensor identity. Their event
callbacks write only to a synthetic in-memory list. It does **not** construct or
open Computer/SuperIO, register a service or load/install a driver.

Tests cover the original intermediate zero, prepared 100% acquisition, repeated
duties, duplicate/unowned/foreign-mode rejection, invalid duties, pump limits,
idempotent release, failed acquisition/write, retained ownership after failed
restoration and a real retry despite LHM's cached Default state. The test-only
factory exists solely under the `NATIVE_CONTROL_TESTS` compile symbol, never in
the production helper. Broker acceptance separately verifies the production
factory rejects an unprotected process and contains no test factory.

The checks pass locally in PowerShell 7 and Windows PowerShell 5.1. The quick
Mainboard native control adapter workflow runs independently of Rust builds;
Windows CI and Package also include it. Full native register/firmware behavior,
cross-process hardware exclusivity and independent recovery remain open. Existing
API types/config files and both GUI boundaries are unchanged.

## Support tiers (DECISIONS.md #14)

`NativeControl` now has a requested tier. `Verified` (default, all legacy paths)
still requires `NativeNctEvidence`. `CreateTieredProtected` admits a control
without audited register evidence only for an explicitly authorized
`LhmSupported` or `Unrestricted` tier, and only when the evidence is missing for
lack of an audited scheme (`NativeEvidenceUnavailableException`); vendor-check,
manual-mode or ISA-mutex failures still refuse. Floors: 40% for fans in
verified/LHM-supported, 100% for pumps, 0% and pump reduction only for an
unrestricted unlock (pump reduction additionally needs `pump_unlocked`). The
foreign-software-mode refusal and the prepared no-zero acquisition apply to every
tier. Without readback `FirmwareReturnVerified` stays false; `FirmwareReturned`
reports the owned `SetDefault` callback with LHM's Default mode. Telemetry for
lower tiers uses the same checked ISA-bus wrap (`NativeIsa`) as the verified path.
Tests: `providers/mainboard/tests/NativeTierTests.cs` (synthetic only).
