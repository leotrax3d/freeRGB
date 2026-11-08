# Release fix plan (docs/legal)

Owner: LEGAL-LEAD. Created 2026-11-07. This plan tracks engineering work only.
It is not legal advice and claims no legal approval; items that need a lawyer or
owner decision are listed in `OWNER_DECISIONS.md`.

Status legend: `todo`, `wip`, `merged`, `owner` (needs decision), `hw` (hardware
acceptance, see `docs/promts/fancontrol/07-hardware-acceptance.md`), `coord`
(owned by the coordinator).

## Handled elsewhere (do not duplicate)

| Finding | Package | Branch | Status |
|---|---|---|---|
| FR-001/002 (P0.1), FR-003/004 (P0.2), FR-014 (P0.6) | coolsafety | task/coolsafety | coord |
| FR-008/009 (P1) | updater | task/updater | coord |

Lead files must not be edited until those branches are merged:
`crates/freergb-control/**`, `crates/freergb-daemon/src/{hwmon,gpu_cooling}.rs`,
`crates/freergb-modules/src/cooling_host.rs`, `crates/freergb-update/**`,
`docs/SECURITY.md`, `docs/UPDATES.md`.

## Packages

| Pkg | Findings | Files (scope) | Depends on | Status |
|---|---|---|---|---|
| notices | FR-007, FR-018 (SBOM/notice part), FR-010 (tooling + gap report) | `NOTICE`, `THIRD_PARTY_NOTICES.md`, `scripts/gen-notices.py`, `docs/legal/SBOM*`, `packaging/windows/freergb.iss`, `packaging/linux/**` notice install, source-offer file | none | done on task/legal-notices, awaiting merge |
| fr005 | FR-005 (P0.3) | `apps/desktop/src/features/onboarding/safety.ts`, `docs/COOLING_SAFETY.md`, `providers/mainboard/NativeControlSession.cs` comment text, wizard text version; en/de locale only | `docs/SECURITY.md` part waits for updater merge | todo |
| privacy | FR-006 (P0.4 engineering part) | `crates/freergb-effect-host/src/{ratings,marketplace}.rs`, `appwrite/community/**`, `docs/WEBSITE_CONTENT.md`, `docs/PRIVACY.md` (new draft), offline/no-count switch | none | wip (branch task/legal-privacy, not merged; owner items open) |
| linux-ci | FR-013 (done), FR-011 | `packaging/linux/60-freergb.rules`, a fixture test, `.github/workflows/modules.yml`, release allowlist file | none | merged (allowlist `packaging/modules/release-allowlist.txt`, only razer-chroma until provenance review) |
| docs | FR-015, FR-016, FR-017, FR-018 (test plan), P0.5, P0.7 | `docs/legal/OWNER_DECISIONS.md`, `docs/legal/RELEASE_FIX_PLAN.md`, `docs/promts/fancontrol/07-hardware-acceptance.md` | none | wip (lead) |
| fr012 | FR-012 | `crates/freergb-daemon/src/{api,onboarding,hwmon,main,cooling_sessions,cooling_plugins}.rs` | coolsafety merged | implemented; fmt, workspace clippy and workspace tests passed (Windows; mocks) |

## Per finding

| Finding | Severity | Package | Result expected | Status |
|---|---|---|---|---|
| FR-001/002/003/004/014 | HIGH | coolsafety | coordinator | coord |
| FR-005 | HIGH | fr005 | Claims match code per path; text version bumped | merged |
| FR-006 | HIGH | privacy | Data flow documented, counting opt-out/offline, retention, privacy notice draft | wip: `docs/PRIVACY.md` draft + data-flow table; `online`/`count_downloads` switches (both default off, tests prove no identity/receipt when off); claims fixed in website content, README, user guide. Open (owner D4/D8): controller, legal bases, Appwrite DPA, retention periods, erase-by-user and access procedures (none exist), website Google Fonts request |
| FR-007 | HIGH | notices | Generated notices from resolved graph, installer/package include them, SBOM without NOASSERTION where derivable | done on branch: `scripts/gen-notices.py` builds `THIRD_PARTY_NOTICES.md` (580 crates, 54 npm, license texts where packaged); installer and Linux staging install `NOTICE`, notices and draft `SOURCE_OFFER.md`; SBOM licenses filled for the shipped graph. Still open: payload extraction/verification, system libraries, LHM closure, crates without license files, lawyer review (D2) |
| FR-008/009 | MED | updater | coordinator | coord |
| FR-010 | MED | notices | Provenance generator + gap report; per-module upstream review is manual | tooling done on branch: `scripts/check-module-provenance.py` writes `MODULE_PROVENANCE_GAPS.md` (heuristic; 27 of 95 records under 15 lines). Manual per-module upstream review and PROVENANCE edits still open (owner, D3) |
| FR-011 | MED | linux-ci | Allowlist-driven build and source job | merged (guard now allowlist-driven; extending it needs FR-010 review) |
| FR-012 | MED | fr012 | Revoke drains sessions and releases hwmon, GPU and plugin channels; saved hwmon assignments require current safety confirmation. Mock release and restart regression coverage. | implemented; fmt, workspace clippy and workspace tests passed (Windows; mocks) |
| FR-013 | MED | linux-ci | Exact pwmN / pwmN_enable only, fixture test | merged (scripts/test-udev-pwm-glob.sh) |
| FR-015 | UNCLEAR | docs | Lawyer brief + file list; no clean-room claim | owner |
| FR-016 | UNCLEAR | docs | Applicability matrix draft (CRA, product liability) | owner |
| FR-017 | UNCLEAR | docs | Asset provenance checklist; trademark/patent search tasks | owner |
| FR-018 | HIGH | docs | Release-SHA checklist; artefact extraction and hardware steps | hw / owner (SBOM/notice part done on task/legal-notices; payload SBOM, advisory scan and hardware tests still open) |

Release key, signing, publishing and real-hardware tests are out of scope here.

## Merge gate

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked` (desktop tests if touched). No CI wait.

## Schedule

Codex workers are unavailable until about 15:43 on 2026-11-07. Wave 1 (max 3 at
once): notices, fr005, privacy. Wave 2: linux-ci, then fr012 once coolsafety is on main.

## State at shutdown (2026-11-07)

On main: plan, OWNER_DECISIONS.md, FR-013, FR-011. Pushed, NOT merged, none passed the full
`cargo test --workspace --locked` gate yet (fmt and clippy passed for notices and privacy):

| Branch | Content | Next |
|---|---|---|
| task/legal-notices | gen-notices.py, THIRD_PARTY_NOTICES.md, NOTICE, SOURCE_OFFER.md (draft), installer/Linux staging, SBOM licences, provenance gap report | run workspace tests, review, merge. Not run: Inno compile, desktop npm tests |
| task/legal-fr005 | WIP safety claims text (safety.ts, COOLING_SAFETY.md, NativeControlSession.cs comments, onboarding.rs, mock transport) | review the diff, bump/verify text version, run full gate incl. desktop tests; mark items pending task/coolsafety |
| task/legal-privacy | community.json switches (online/count default off), GUI toggle, docs/PRIVACY.md draft, truthful README/website text | run workspace tests, merge |
| fr012 | not started | start after task/coolsafety is on main |

Still to do: OPEN_WORK.md, HANDOFF.md entries, hardware items into
`docs/promts/fancontrol/07-hardware-acceptance.md`. Merge order suggestion: notices, privacy, fr005
(fr005 and privacy may both touch locale files and onboarding code; check conflicts).

FR-005 branch notes (task/legal-fr005, 66d9b66): safety text v2 per backend, pinned in mock and
tests; claims-to-evidence table and unsafe-exception section in COOLING_SAFETY.md. cargo test and
npm/vitest NOT run (no node_modules there). Conflict risks: the translations branch also edits
safety.ts; fr012 edits onboarding.rs. FR-005 row stays `wip` until the gate passes.
