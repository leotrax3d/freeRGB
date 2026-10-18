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
| 9 | GUI (egui), plugin system | ⏭ next |
| 10 | More vendors (iCUE LINK, ASUS AURA, Razer) | open |
| 11 | Effects library, layout editor, profiles, screen ambience, audio, effect editor | open |
| 12 | Windows/Linux compatibility (CI) | open |

Deviation with reason: discovery and the adapter registry were built before the
daemon because a safe first hardware validation needed them (only known interfaces
are ever written to).
