# Prompt — OpenRGB coverage: finish open ports, then the next HID families

Independent of the fan-control work. Read `docs/HANDOFF.md`,
`docs/OPENRGB_PORTING.md`, `docs/OPENRGB_COMPATIBILITY.md`,
`docs/HARDWARE_MODULES.md` and `docs/DECISIONS.md` first.

## Rules
- One branch per family from `main` (`module/<family>`), own worktree, one PR
  each; delete the branch after merge. Never start a second implementation of
  a family that already exists in `modules/`.
- Ported OpenRGB code is GPL-2.0-or-later and lives only in its process module
  under `modules/` (PROVENANCE.md, COPYING, pinned OpenRGB commit). Core stays
  MIT/Apache-2.0.
- Do not touch cooling code (`crates/freergb-control`, `crates/freergb-daemon/src/cooling_*`,
  `providers/mainboard`) — another workstream owns it.
- No hardware access; byte-exact report tests against the upstream source.
- Regenerate the compatibility matrix (`scripts/openrgb/compat-matrix.mjs`) and
  keep the drift check green. Public demo data must not contain real brand names.
- Repo language English.

## Tasks
1. Finish `module/mountain-keyboard` and `module/thingm-blink` (WIP branches on
   GitHub, never built or tested): rebase on `main`, build, tests, PR each.
2. Then take the next unported HID families from the priority table in
   `docs/OPENRGB_PORTING.md` (highest device count first), one PR per family,
   until told to stop. Record each port in the ledger.

## Done when (per family)
Module builds, report tests byte-exact, `--list-devices` works, matrix updated,
CI green, PR open; status "ported (simulated)", never hardware-accepted.
