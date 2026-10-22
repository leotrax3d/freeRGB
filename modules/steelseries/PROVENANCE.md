# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/SteelSeriesController/`. Detector data is generated from the
pinned `SteelSeriesControllerDetect.cpp` and includes the callback family for
each VID/PID. freeRGB uses brokered HID and never reads serial numbers or paths.
