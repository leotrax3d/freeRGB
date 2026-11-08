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
| `modules.forget` | no UI to remove a configured device from setup | P1 `setup-forget` |
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
| P1 setup-forget | `features/setup/*`, en/de keys | in progress |
