# Cooling validation ledger

Status 2026-10-20: stage 1 in progress. No cooling-duty writes have been sent.
The existing curve editor remains simulation-only. Cached LINK temperatures are
implemented and simulator-tested; live acceptance of this change is pending.

## Evidence

* Historical local hardware observations: `DEVICES.md`, `PROTOCOLS.md` (2026-10-18).
  Hub VID:PID 1b1c:0c3f, firmware 4.1.144, raw RPM type 0x0025, temperature type
  0x0010, status/value triples. These are historical, not fresh measurements.
* Installed SignalRGB `Corsair/Modern_Corsair/Corsair_ICUE_Link_Hub.js` SHA256:
  `8392de29c4a561295df30c400b1008cfe38a392cf470c0665129e797eb64e5bf`.
  Inspected `FetchTemperatures`, sensor assignment and duty endpoint definitions.
  Its positional assignment of nonzero sensors is not sufficient physical proof.
* [OpenLinkHub lsh.go, pinned commit c8c1d2a](https://github.com/jurkovic-nikola/OpenLinkHub/blob/c8c1d2a99a56aa1d907e43137d127c2048f41ba5/src/devices/lsh/lsh.go):
  `getDevices` uses one-based channel numbering; `getDeviceData` indexes telemetry
  by channel; temperature is signed tenths Celsius. Endpoint 0x18 uses data type
  0x0007 and channel/mode/duty/reserved records. This establishes a protocol candidate,
  not hardware validation. Only protocol facts were used; no GPL source copied into
  product code. Cached upstream source is under ignored `target/cooling-evidence/`.

## Candidate channel map, not physically confirmed

| Child slot | Historical child | Candidate telemetry channel |
|---|---|---|
| 0, 1, 2 | RX fans | RPM 1, 2, 3 |
| 3 | Titan 360 RX AIO | RPM/temperature 4 |
| 12 | Commander DUO | 13; attached physical function unknown |
| 13 | LX fan | RPM 14 |
| 14 | Commander DUO | 15; attached physical function unknown |

Candidate = child slot + 1, corroborated by the upstream protocol implementation
and historical local values. It is displayed explicitly as unconfirmed. In
particular, temperature 4 must not be silently labeled confirmed coolant or used
as the control source yet. Unknown/absent channels remain unknown.

## Implemented checks

* Strict telemetry header/type/count/length validation; no partial table accepted.
* Signed Celsius conversion and unavailable markers for implausible values outside
  -20..125 C. These are data sanity bounds, not recommended thermal operating limits.
* Temperature polling alongside the existing two-second RPM watchdog in the device
  worker. Cache reads perform no USB I/O; errors retain the original sample age.
* Refuse software sessions without a valid spinning RPM baseline. RPM watchdog
  continues even if temperature endpoint is unavailable.
* Restore only sessions freeRGB owns; retry failed restoration during disconnect;
  attempt cleanup on normal Rust drop. Other programs' software sessions are not
  reset. Drop is not an independent supervisor and does not handle process kill,
  deadlock or USB failure.

## Remaining gates before live curves

1. SignalRGB/other active controllers must be closed. Fresh short sensor-only
   software window; save raw report and verify hardware-mode restoration.
2. Correlate child identity, RPM and physical fan/pump; verify temperature source.
   Do not identify pump/coolant from a plausible number alone.
3. Establish exact firmware-specific duty semantics, update scope and pump limits.
   A valid-looking command acknowledgement does not prove correct speed behavior.
4. Implement a separate supervisor/lease with verified fallback on control-worker
   stall, process crash, stale/missing sensor, critical temperature and failed I/O.
   Require physical restore/readback tests, not just simulation.
5. Only then enable bounded reversible duty validation, preserving pump operation
   and restoring hardware mode. Verify actual RPM response and restored behavior.
6. Bind saved curves only to validated channel/source identities, enforce pump
   constraints and test unplug/replug, sleep/resume and longer soaks.

No guessed control packets, CPU/GPU readings, automatic activation or fan/pump
stop commands are part of this stage. The six-stage user order remains cooling,
installer, updates, macros, monitoring, then RGB/profiles. Later stages are not
marked complete while this gate is open.
