# Open work (2026-11-07)

Collected from `main` (HANDOFF.md, STATUS.md, docs/promts/, code TODOs), all
remote branches and open PRs. Done and duplicate items omitted. Code TODOs: none
outside `vendor/` (vendored glib, not ours). Device ports in `modules/` are done
by another person and are listed only for reference.

Legend: HW = needs real hardware/UAC/installer/release key (collect in
`docs/promts/fancontrol/07-hardware-acceptance.md`, do not implement here).

## Software (can be done here)

| # | Item | Area | Files | Depends on | Source | HW/UAC |
|---|---|---|---|---|---|---|
| S1 | Finish non-HID transport host (A7): SDK protocol/serial/network transports, store/catalog fields, fixture binary, tests. Branch is unfinished WIP, never verified | module API | `crates/freergb-module-api/src/transport*.rs`, `crates/freergb-modules/src/{transport_host,catalog,package,store}.rs`, `crates/freergb-modules/tests/transport.rs` | A1–A4 (on main) | `origin/wip/ofc-a7-transports` | no |
| S2 | A5 keyboard layouts / matrix maps (shared KeyboardLayoutManager equivalent) | module API | `crates/freergb-module-api/` (new layout module), docs | none | `docs/promts/herdr-openrgb-full-coverage.md` Phase A | no |
| S3 | A6 multi-interface devices + HID++ / receivers (one logical device over several interfaces) | module API | `crates/freergb-module-api/`, `crates/freergb-modules/src/` host side | conflicts with S1 in `freergb-modules` host files → sequence after S1 or split files | same | no |
| S4 | Make `tests/effect_marketplace.rs` self-sufficient (builds/locates effect-worker itself; `cargo test -p freergb-daemon` alone must pass) | tests | `crates/freergb-daemon/tests/effect_marketplace.rs` | none | HANDOFF follow-ups | no |
| S5 | De-flake marketplace and plugin process tests under load (looser/conditional timing) | tests | `crates/freergb-daemon/tests/*plugin*`, `effect_marketplace.rs` | do together with S4 | HANDOFF follow-ups | no |
| S6 | Translate cooling, macros and mainboard-RGB views (en/de) via existing i18n | desktop GUI | `apps/desktop/src/**` cooling/macros/mainboard views + locale files | none | HANDOFF WS5 | no |
| S7 | AMD GPU fan/sensor source via ADLX (simulated tests) | cooling | `crates/*cooling*` sources, `docs/COOLING_SOURCES_GPU.md` | none | HANDOFF gap 3, prompt 09 | impl no, acceptance HW |
| S8 | Modern-GUI live cooling-session parity with Classic (public session API in Tauri) | desktop GUI | `apps/desktop/src/**` cooling, Tauri commands | conflicts with S6 in cooling view → same package or sequence | STATUS Motherboard cooling | impl no, acceptance HW |
| S9 | Legal/safety audit PR #19: review and merge docs (`docs/legal/`) | docs | `docs/legal/**` (docs only) | none | PR #19 `audit/legal-safety-2026-11-05` | no |
| S10 | Audit P0.6 (before merging new API3 plugins): short-write/hung-driver bound — 0/short/error transfers must not report fail-safe success | cooling plugins | cooling plugin host crate | S9 | audit LEGAL_ACTION_PLAN | no |
| S11 | Audit P0.1: durable recovery transaction for GPU/hwmon writes; missing journal persistence blocks I/O | cooling | GPU/hwmon backends, recovery journal | S9 | audit | no (mocks) |
| S12 | Audit P0.2: backend-independent emergency control + temperature interlock in Manual mode (hang/kill/suspend/missing sensor → safe state within bound) | cooling | cooling runtime/supervisor | S9; avoid same files as S11 | audit | no (simulated) |
| S13 | Audit P1 FR-012/013: consent revocation stops active control, no auto-resume; exact Linux file permissions (no extra chmod bits) | cooling / Linux packaging | consent handling, Linux packaging/udev scripts | S9 | audit | no |
| S14 | Audit P1 FR-008/009: independent update signature (online metadata alone not enough for elevated code); correct sandbox claims in docs | updates | updater crate, `docs/UPDATES.md`, `docs/SECURITY.md` | S9 | audit | no |
| S15 | Audit P0.3: correct safety claims in wizard/docs to what is actually implemented | docs/GUI text | `docs/COOLING*.md`, guided setup texts | after S11/S12 | audit | no |
| S16 | Linux screen-capture CI step (snippet in PR #29) — add to `.github/workflows/ci.yml` | CI | `.github/workflows/ci.yml` | none (CI billing currently off) | HANDOFF WS3 | no |
| S17 | i18n preparation: JSON locale catalogs, CLDR plurals, RTL, font fallbacks, validation script, pseudo-locale, language picker with machine-translated marker, English safety warnings, `docs/I18N.md` | desktop GUI / scripts | `apps/desktop/**`, `scripts/`, `docs/I18N.md` | after S6 merged | owner | no |
| S18 | Translate catalogs to ~50 languages (one BEAM agent per language, batches of 8-10; text only), all pass S17 validation, marked machine-translated | locales | `apps/desktop/**/locales/*.json` | after S17 | owner | no |
| S19 | Classic GUI small i18n from same JSON, daemon/tray error codes translated in GUI, USER_GUIDE and website guide pages via BEAM (developer docs stay English) | classic GUI / docs | classic GUI crate, `docs/USER_GUIDE.md`, website guides | after S18 | owner | no |

## Not for this machine / owner decisions

| Item | Source | Why |
|---|---|---|
| Phase B OpenRGB family ports (~1055 detectors), `module/mountain-keyboard`, `module/thingm-blink` | full-coverage / simple-ports prompts, branches | device ports are done by another person (`modules/`) |
| Audit P0.4 (online data flows/legal basis), P0.5 (license/role/liability text), FR-017 (name/assets/patents), P0.7 release checks | audit | legal/owner decision, release SHA |
| Signed module/effect catalog with real key, published release | STATUS Distribution | release key |
| GitHub Actions billing | HANDOFF | owner account |

## Hardware / UAC (collect in 07-hardware-acceptance.md only)

R5 fine sweep, R5b, R5c re-tests; blocks B–G; keyboard typing/foreground profile;
LINK soak; ASUS header colour/fallback; Razer module; macro output/hotkeys and
per-device triggers; sensor/NVML/ADLX acceptance; cooling plugin on iCUE LINK;
service/boot/driver-hang recovery; maintenance upgrade/uninstall/rollback;
installer clean-machine/UAC/reboot; cross-version update; Linux hardware;
German UI/screen reader check and screenshots; website proofread.

## UI-LEAD (Tauri UI gaps)

- Inventory: `docs/UI_GAPS.md`. P1 setup-forget merged. Open: re-audit cooling/devices after guicool + S22; a11y/state pass; English INTEGRATION_CHECKLIST; screenshots.

## Legal release fixes (2026-11-08)

Merged: FR-005/006/007/011/012/013 and the legal drafts. Open: D3 provenance rewrites, Appwrite re-auth and approval, lawyer/insurance review, release-SHA checks. See [RELEASE_FIX_PLAN.md](legal/RELEASE_FIX_PLAN.md) and [OWNER_DECISIONS.md](legal/OWNER_DECISIONS.md).
