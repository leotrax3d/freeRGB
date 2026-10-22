# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/SteelSeriesController/`. Detector data is generated from the
pinned `SteelSeriesControllerDetect.cpp` and includes the callback family for
each VID/PID. freeRGB uses brokered HID and never reads serial numbers or paths.

## Audit correction

The former claim was incorrect. OpenRGB dispatches the 76 detector rows to 17
controller-specific implementations. The Rust module had only a generic
64-byte packet and no complete initialization, LED mapping, direct-mode and
cleanup equivalence, so all 76 rows are explicitly **nicht portiert** and
`identify()` returns `supported=false` for them.

| Upstream path | Devices | Rust mapping | Status |
|---|---:|---|---|
| Aerox 3/5/9 (`SteelSeriesAerox*Controller`) | 13 | detector table only | nicht portiert |
| Rival (`SteelSeriesRival*Controller`) | 20 | detector table only | nicht portiert |
| Sensei (`SteelSeriesSenseiController`) | 4 | detector table only | nicht portiert |
| Siberia / Arctis (`SteelSeriesSiberiaController`, `SteelSeriesArctis*Controller`) | 4 | detector table only | nicht portiert |
| QCK (`SteelSeriesQCKMatController`) | 9 | detector table only | nicht portiert |
| Apex (`SteelSeriesApex*Controller`, old Apex) | 26 | detector table only | nicht portiert |

These six rows cover all 17 upstream sub-controller directories and all 76
generated device entries in `src/devices_table.rs`. No packet test is retained
for the removed generic report; packet tests will be added with each real
controller port.
