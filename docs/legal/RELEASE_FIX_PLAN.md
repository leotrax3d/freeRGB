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
| notices | FR-007, FR-018 (SBOM/notice part), FR-010 (tooling + gap report) | `NOTICE`, `THIRD_PARTY_NOTICES.md`, `scripts/gen-notices.py`, `docs/legal/SBOM*`, `packaging/windows/freergb.iss`, `packaging/linux/**` notice install, source-offer file | none | todo |
| fr005 | FR-005 (P0.3) | `apps/desktop/src/features/onboarding/safety.ts`, `docs/COOLING_SAFETY.md`, `providers/mainboard/NativeControlSession.cs` comment text, wizard text version; en/de locale only | `docs/SECURITY.md` part waits for updater merge | todo |
| privacy | FR-006 (P0.4 engineering part) | `crates/freergb-effect-host/src/{ratings,marketplace}.rs`, `appwrite/community/**`, `docs/WEBSITE_CONTENT.md`, `docs/PRIVACY.md` (new draft), offline/no-count switch | none | todo |
| linux-ci | FR-013, FR-011 | `packaging/linux/60-freergb.rules`, a fixture test, `.github/workflows/modules.yml`, release allowlist file | none | todo |
| docs | FR-015, FR-016, FR-017, FR-018 (test plan), P0.5, P0.7 | `docs/legal/OWNER_DECISIONS.md`, `docs/legal/RELEASE_FIX_PLAN.md`, `docs/promts/fancontrol/07-hardware-acceptance.md` | none | wip (lead) |
| fr012 | FR-012 | `crates/freergb-daemon/src/{api,onboarding,hwmon,main}.rs` | coolsafety merged (hwmon.rs conflict) | todo (blocked) |

## Per finding

| Finding | Severity | Package | Result expected | Status |
|---|---|---|---|---|
| FR-001/002/003/004/014 | HIGH | coolsafety | coordinator | coord |
| FR-005 | HIGH | fr005 | Claims match code per path; text version bumped | todo |
| FR-006 | HIGH | privacy | Data flow documented, counting opt-out/offline, retention, privacy notice draft | todo (legal basis: owner) |
| FR-007 | HIGH | notices | Generated notices from resolved graph, installer/package include them, SBOM without NOASSERTION where derivable | todo |
| FR-008/009 | MED | updater | coordinator | coord |
| FR-010 | MED | notices | Provenance generator + gap report; per-module upstream review is manual | todo (partly owner) |
| FR-011 | MED | linux-ci | Allowlist-driven build and source job | todo |
| FR-012 | MED | fr012 | Revoke stops channels; startup honours safety state | blocked |
| FR-013 | MED | linux-ci | Exact pwmN / pwmN_enable only, fixture test | todo |
| FR-015 | UNCLEAR | docs | Lawyer brief + file list; no clean-room claim | owner |
| FR-016 | UNCLEAR | docs | Applicability matrix draft (CRA, product liability) | owner |
| FR-017 | UNCLEAR | docs | Asset provenance checklist; trademark/patent search tasks | owner |
| FR-018 | HIGH | docs | Release-SHA checklist; artefact extraction and hardware steps | hw / owner |

Release key, signing, publishing and real-hardware tests are out of scope here.

## Merge gate

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked` (desktop tests if touched). No CI wait.

## Schedule

Codex workers are unavailable until about 15:43 on 2026-11-07. Wave 1 (max 3 at
once): notices, fr005, privacy. Wave 2: linux-ci, then fr012 once coolsafety is on main.
