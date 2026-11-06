# Provenance
Ported from OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/AsusLegacyUSBController`.

## Audit correction

OpenRGB has three separate device paths: Cerberus keyboard, Sagaris keyboard,
and Strix Claw. The Rust module only contained a generic one-colour shortcut
and did not port their initialization, layout, direct mode, or cleanup. All
three detector rows are **nicht portiert** and `identify()` returns
`supported=false`.

| Upstream device/path | Rust mapping | Status |
|---|---|---|
| ASUS Cerberus Mech / `AsusCerberusKeyboard` | detector table only | nicht portiert |
| ASUS Sagaris GK1100 / `AsusSagarisKeyboard` | detector table only | nicht portiert |
| ASUS ROG Strix Claw / `AsusStrixClaw` | detector table only | nicht portiert |
