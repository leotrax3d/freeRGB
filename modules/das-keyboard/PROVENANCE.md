# Provenance
Ported from OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/DasKeyboardController`.

## Audit correction

The previous claim was incorrect. OpenRGB's Das Keyboard path includes
device-specific initialization, per-key LED mapping, direct mode, and cleanup
in `RGBController_DasKeyboard.cpp`. The Rust code only emitted a single-colour
shortcut and does not complete that path. All three generated device rows
(`Q4 RGB` 24f0:2037, `Q5 RGB` 24f0:2020, and `Q5S RGB` 24f0:209a) are
**nicht portiert**; `identify()` returns `supported=false`.

| Upstream path / device | Rust mapping | Status |
|---|---|---|
| `RGBController_DasKeyboard` / Q4 RGB | detector table only | nicht portiert |
| `RGBController_DasKeyboard` / Q5 RGB | detector table only | nicht portiert |
| `RGBController_DasKeyboard` / Q5S RGB | detector table only | nicht portiert |
