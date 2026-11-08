# S22: shared Headers setup and vendor conflict handling

Branch: `task/mbheaders`.

## Changes

- Added a dedicated Headers navigation page to Tauri and Classic; removed the
  older embedded motherboard panel from Devices/Monitoring.
- Shared daemon settings now persist header type, LED count, name, colour order
  and a normalised shared-canvas rectangle. RGB uses one colour slot. Old
  settings retain their defaults. Import/export validates the new fields.
- IT5711 `048d:5711` accepts six settings slots, including both onboard slots;
  other controller keys retain the four-slot settings bound. Both GUIs and the
  mock transport preserve all six slots.
- Added the timed `mainboard_rgb.test` RPC: red, green and blue, two seconds each,
  followed by normal effects. Zero-length/invalid headers and vendor conflicts
  are rejected. Rendering uses ASUS detection ranges or adapter-named zones,
  including sparse Fusion slots and ASRock's interleaved onboard zones.
- ASUS, Gigabyte, ASRock and MSI appear in the shared panel. MSI fixed zones
  expose their actual names/counts/types from the connected module; unsupported
  resizing is disabled. Per-header order and canvas mapping run before device
  colour correction and protocol encoding; ASUS keeps its native order path.
- Vendor conflicts release active motherboard devices and also prevent module
  attachment while conflicts or the global management switch forbid it.
  Tauri refreshes warning state while the page is open.
- Added the missing Tauri allow-list entries for motherboard get/save/test.
  Added a queue regression check including `keyboard.reactive.preview`.
- Only `en.json` and `de.json` were changed; no other catalog or metadata changes.

## Validation

- `scripts/gate.ps1 -Fast -Package freergb-daemon,freergb-gui`: passed (fmt, clippy, nextest, doctests, 8 cooling-guard fixtures and 5 bus-process fixtures).
- `cd apps/desktop; npm.cmd test`: passed, 30 files / 143 tests.
- `cd apps/desktop; npm.cmd run typecheck`: passed.
- Tauri queue: isolated Cargo harness compiled the actual `queue.rs` with its
  real client dependency; all 5 queue tests passed. Full Tauri packaging was
  not run. Tauri Rust formatting checked separately.
- Tests were simulated only. No UAC, hardware/bus access, passwords, agents or
  module ports. Owner-only acceptance entries are in
  `docs/promts/fancontrol/07-hardware-acceptance.md` (H1-H5).
- The initial E: build ran out of space. The gate was rerun with target directory
  `D:/freergb-build/mbheaders-s22`, debug symbols/incremental compilation disabled
  and sccache disabled. One generated E: library was preserved under that
  directory's `displaced` subdirectory to recover working space.

## Integration notes

- P1/P2 protocol ports are separate work. The six-slot settings and GUI support
  do not by themselves enable an unsupported IT5711 driver; P2 must consume all
  six settings slots when constructing its adapter.
- MSI modules that currently expose fixed zones cannot accept arbitrary strip
  lengths; the shared page honours the reported capabilities instead.
- The keyboard UI has no preview action in `KeyboardReactions.tsx`; that file
  was not touched. Its daemon RPC and Tauri allow-list entry already exist.
  The missing keyboard UI action remains a follow-up, as requested by t12k.
- The optional cooling-channel name link was not added.
