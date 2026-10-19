# Cooling status

The curve editor/API is a simulation and policy implementation, not live fan control.
No new cooling write command has been sent to real hardware.

`freergb-control::FanCurve` validates 2–16 strictly ordered temperature points with
nondecreasing duty, minimum duty 20–100%, critical threshold 40–110 °C and downward
hysteresis 0–10 °C. Linear interpolation uses endpoints outside the point range.
Missing/nonfinite/implausible/stale (>3 s) temperatures or invalid policy select
100% in the policy model. Critical temperature bypasses hysteresis. These defaults
are not validated model-specific pump/fan limits and are not sent to devices.

Named curves live next to persisted state in `control.json`. `--no-state` keeps
the library in memory only. The preview is explicitly labelled simulated.

## Protocol evidence, 2026-10-19

Source: installed SignalRGB plugin
`%LOCALAPPDATA%/WhirlwindFX/SignalRgb/cache/plugin_cdn/main/Plugins/Corsair/Modern_Corsair/Corsair_ICUE_Link_Hub.js`.
SHA-256: `8392de29c4a561295df30c400b1008cfe38a392cf470c0665129e797eb64e5bf`.

Static observations (not hardware confirmation): fan-speed endpoint `0x18`,
fan-duty data type `0x07`, a count followed by per-channel entries including RPM
sensor ID and requested percentage. The plugin refreshes requests periodically
and has a separate pump-specific minimum branch. Its sensor mapping can move
between enumerations; the plugin itself warns against assuming fixed RPM positions.

Open: actual fan/pump channel identity, behavior when software disappears, accepted
duty range per model, firmware-curve restoration, response timing and ownership.
These must be validated before a hardware write adapter or cooling loop is enabled.
No SignalRGB implementation source was copied into this project.
