# Prompt 01 — Motherboard fan coverage matrix, chip simulation, community reports

Read `00-common-rules.md` first. Branch: `cooling/coverage-matrix`.

## Goal
Make "almost every motherboard" a measurable, tested claim: support is defined
per fan-controller chip family, every family is exercised in simulation, and
real-world evidence from other users can be collected and recorded.

## Tasks
1. **Generated coverage matrix** (pattern: `scripts/openrgb/compat-matrix.mjs`).
   - Script that extracts all SuperIO/EC chip families with fan control from the
     pinned LHM version (v0.9.6) and all Linux hwmon drivers freeRGB supports
     (`crates/freergb-daemon/src/hwmon.rs`).
   - Output `docs/FAN_COVERAGE.md` + `docs/fan-coverage.json`: chip, vendor,
     typical board vendors, backend (LHM/hwmon), freeRGB tier
     (verified/LHM-supported/unrestricted/read-only), status
     (hardware-verified/simulated/unsupported), evidence link.
   - CI drift check like `openrgb-drift.yml`.
2. **Chip-family simulators.** Faithful fakes for at least: Nuvoton NCT67xx
   separate-mode, NCT6701D (owner board), NCT668x/NCT6687D (MSI), ITE IT86xx/IT87xx
   (Gigabyte/ASRock), Fintek F718xx, Winbond, ASUS EC sensors (read-only). Model
   quirks: shared mode registers, inverted/limited duty ranges, missing readback,
   delayed RPM response, channels that ignore writes. Run the existing guided
   channel test, runtime plausibility, fault return and unrestricted unlock
   against every family in tests.
3. **Linux hwmon parity.** Same matrix and simulated tests for `pwmN_enable`
   semantics per driver (nct6775, it87, nct6683, f71882fg, asus-ec-sensors).
4. **Community hardware report.** Extend the redacted diagnostics export with a
   cooling section (board, BIOS, chip, channels, tier, channel-test results; no
   serials/paths/user names). Add a GitHub issue template "Fan control hardware
   report" and a script that turns accepted reports into matrix evidence.
5. GUI/diagnostics show the chip, tier and coverage status per channel.

## Done when
Matrix generated and drift-checked; every listed family has simulated tests for
test/plausibility/fault/unlock paths; report export + template exist; only the
owner board (NCT6701D) may be marked hardware-verified, and only for what
`COOLING_ACCEPTANCE.md` proves.
