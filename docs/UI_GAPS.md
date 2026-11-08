# Tauri UI gaps (apps/desktop)

Inventory by UI-LEAD, 2026-11-08. Rule: no redesign. Only completeness,
correctness, consistency inside the existing design system, usability, bugs.
Sources: `docs/LOCAL_API.md` methods vs. string literals in `apps/desktop/src`,
`docs/INTEGRATION_CHECKLIST.md`, `docs/GUI_MODERNIZATION.md`.

## Reserved areas (do not touch until merged)

- `task/guicool`: cooling views (`features/cooling/*`), `lib/api/transport.ts`.
- `task/translations`: `lib/i18n/locales/*` (other locales), `locale-metadata.json`.
- S22 Headers page: `features/devices/*` mainboard RGB area.

## Daemon API not used by the Tauri GUI

| Method | Status | Package |
|---|---|---|
| `modules.forget` | done: ConfirmButton on configured setup cards | P1 merged |
| `keyboard.reactive.preview` | no preview action in KeyboardReactions (devices area, reserved) | deferred to S22 |
| `cooling.profiles.export/import/get`, `cooling.providers.list`, `cooling.service.get/set`, `cooling.bindings.get`, `cooling.channels.list` | cooling area, check after guicool merge | deferred |
| `daemon.shutdown*`, `daemon.version`, `lighting.status`, `monitoring.sensors.list` | reached via Tauri commands/snapshot, not direct | none |
| Classic-only: hotkeys, application profiles, updates | launch Classic from Tauri (documented decision) | none |

## Accessibility / keyboard

Files with zero `aria-` usage: `components/page.tsx`, `components/ui/button.tsx`,
`app/DemoBoundary.tsx`, `features/cooling/SupportBadge.tsx`,
`features/monitoring/ProviderSensors.tsx`, `main.tsx`. Most are presentational
and fine; `ProviderSensors` (table/status) and `SupportBadge` need a review
once guicool is merged. Remaining checks need a real desktop (see below).

## States (empty / error / loading)

Not yet audited per view; needs a visual pass. Open item from
`INTEGRATION_CHECKLIST.md`: full visual operation check on a Windows desktop.

## Docs / screenshots

- `INTEGRATION_CHECKLIST.md` is German; repo text should be English.
- `docs/assets` holds only `project-progress.svg`; no current UI screenshots.

## Packages

| Pkg | Scope | Status |
|---|---|---|
| P1 setup-forget | `features/setup/*`, en/de keys | merged (e71ed61) |

## Follow-ups after guicool / translations / S22 merge

Re-run the method diff, audit cooling + devices states/a11y, add the new i18n
keys to other locales (new keys so far: `setup.card.forget`,
`setup.card.forgetConfirm`; en + de done), translate this doc's German source
`INTEGRATION_CHECKLIST.md`, and capture UI screenshots on a real desktop.

## Status 2026-11-08 (update)

Merged: P1 setup-forget (e71ed61), P2 monitoring empty/loading states + list
semantics + English INTEGRATION_CHECKLIST (b5fba7d), P3 cooling profile
JSON export/import + service startup settings (98fc855).
New i18n keys (en + de done; other locales still need them): monitoring.loading,
monitoring.board.providers, monitoring.board.empty, monitoring.board.unavailable,
monitoring.serviceUnavailable, monitoring.deviceSensorsUnavailable,
monitoring.customUnavailable, cooling.profileService.* (32 keys).
Still open: `keyboard.reactive.preview` (S22 devices area); visual state/a11y pass
on a real desktop; UI screenshots; `cooling.providers.list`,
`cooling.bindings.get`, `cooling.channels.list` appear to be consumed through
the snapshot, not direct calls (no gap confirmed).
