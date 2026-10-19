# Roadmap

| # | Phase | Status |
|---|---|---|
| 1 | Determine tools and environment | ✅ |
| 2 | Investigate SignalRGB | ✅ (base inventory) |
| 3 | Reproducible reverse-engineering procedure | ✅ `REVERSE_ENGINEERING.md` |
| 4 | Platform-independent Rust core | ✅ `freergb-core` |
| 5 | Virtual device + tests | ✅ |
| 6 | Corsair device detection | ✅ K70 CORE, LINK hub, LCD, RAM identified |
| 7 | First real RGB data path | ✅ K70 CORE validated |
| 8 | Background service + hardware abstraction | ✅ `freergbd` (autostart pending approval) |
| 9 | GUI (egui), plugin system | 🔄 GUI/layout editor, TOML descriptions and process-device plugin foundation implemented; real hardware plugins and effect plugins pending |
| 10 | More vendors (iCUE LINK, ASUS AURA, Razer) | open |
| 11 | Effects library, layout editor, profiles, screen ambience, audio, effect editor | ✅ 11 effects + layered custom effects, layout editor, profiles, audio, screen ambience, effect editor |
| 12 | Windows/Linux compatibility (CI) | 🔄 CI green on both; Linux hardware untested |

Deviation with reason: discovery and the adapter registry were built before the
daemon because a safe first hardware validation needed them (only known interfaces
are ever written to).

2026-10-19 continuation: replaced the HID mutex with tested FIFO admission;
Windows release build and virtual daemon/API smoke test pass. Next hardware task:
compare simultaneous K70/hub FPS and stability; no performance claim yet.

Process-plugin milestone: explicit manifests, versioned stdio, fault isolation,
GUI/API start/stop, virtual Rust reference and Windows IPC smoke coverage implemented.
Effect/profile validation and bounded API input added. See `WORKLIST.md` for the
current acceptance checklist and explicit hardware/platform backlog.

Scope expanded by user: cooling, fan curves, macros, monitoring, installer and updates.
Full plan/status: `PRODUCT_PLAN.md`. Curve simulation, manual macros, cached LINK RPM,
Windows installer build and release updater foundations exist; actual cooling writes,
global macro triggers, installation/upgrade acceptance and live updates remain open.
