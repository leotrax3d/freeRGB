# Protected native session bridge (commissioning only)

Updated 2026-10-22. The actual end-to-end path passed on ASUS PRIME X870-P WIFI,
NCT6701D, CHA_FAN2: Rust curve host -> user relay -> authenticated SYSTEM service
-> dedicated protected native worker -> checked register write -> verified original
firmware return and empty durable journal. An 80% curve and 100/80/60/40% calibration also ran and returned.
The 50% recommendation remains hardware_verified=false; no FanControl parity is claimed.

## Executable path

`freergb-cooling-pilot <bundled freergb-mainboard.exe>` invokes `CoolingHost` and
`native_bridge`, not GUI policy. The helper's `--control-relay` uses the existing
restricted Win32 pipe rights and verifies SYSTEM pipe ownership, SCM registration
and the protected service path. Rust never opens that pipe with generic-write.

The service owns the exclusive `freergb-mainboard-control-v1` pipe with its existing
account-specific ACL. It launches a fixed-path protected native worker, verifies
its pipe owner and exact process ID, and proxies bounded private protocol records.
The worker acquires only after authenticating the registered SYSTEM service peer.
The native journal independently rejects unresolved acquisition. Readers skip
recovery of a still-living, same-service-lifetime control worker; they never clear
its reservation. After worker death, existing checked recovery applies.

The native worker binds exact temperature and RPM IDs on the selected SuperIO
hardware. It refreshes LHM under a checked ISA lock before enumerating sensors
(some are activated only by that first refresh). A separate sampler refreshes
every100ms; the existing native lease/interlock checks independently. Driver hangs
are not solved by thread cancellation and remain a separate acceptance limit.

Private session replies optionally add `telemetry` with `sequence`, `temperature_c`,
`rpm`, and `age_ms`. Acquisition identity is not a command sequence. Rust validates
range/age/order, anchors acquisition conservatively before reading the response,
and preserves the original local time for repeated sample identities. New command
receipts cannot manufacture fresh measurements. Native runtime sampling is taken
from these authenticated replies; GUI-supplied values are not used.

## Explicit temporary authorization

Normal setup does not create authorization. The protected file
`%ProgramFiles%/freeRGB/mainboard-control-authorization.json` has exactly:

```json
{
  "version": 1,
  "control_id": "/lpc/nct6701d/0/control/2",
  "temperature_id": "/lpc/nct6701d/0/temperature/1",
  "rpm_id": "/lpc/nct6701d/0/fan/2",
  "floor_percent": 100,
  "critical_c": 80,
  "expires_utc": "2026-10-21T06:45:00.0000000Z"
}
```

The timestamp is an expired format example. Authorization must be administrator/
SYSTEM-owned, non-reparse and not user-writable, unexpired and no more than five
minutes ahead. Fan floors 40..100 are accepted with protected role="fan". Reduced pump control is excluded. Sensor labels are still not verified CPU/coolant
identities. The test board's selected temperature is a SuperIO source only.

`scripts/test-mainboard-pilot.ps1 -ConsentToProbe` creates a two-minute protected
authorization for explicit source IDs, runs the Rust pilot, requires a clear
journal after success, and deletes authorization in `finally`. It requires UAC;
it installs no driver or service. `mainboard-pilot-result.txt` is local acceptance
output. `mainboard-control-last-error.json` records version1, stage and error type
without session/ticket secrets. A pre-session worker startup failure may also
produce `mainboard-worker-last-error.txt` for protected local diagnostics.

The additive cooling.sessions API and Classic dialog use this protected path.
Each start requests UAC; the protected service maintenance handshake drains before
replacement/removal. Same-version active replacement passed. Public-path interactive
acceptance, release upgrades/rollback/uninstall and service/OS/driver faults remain open.

Current integration and hardware/release evidence: [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md).
