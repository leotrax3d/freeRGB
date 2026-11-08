# Website redesign report

Branch `task/website-redesign`, merged into `main`. Theme: graphite (owner's choice).

## What changed

- Five pages instead of one long page: Home, Download, Devices, Docs, Privacy.
  Each is a real HTML file, server-rendered at build time and hydrated in the browser.
- shadcn/ui on Tailwind with tokens taken from `apps/desktop`
  ([DESIGN_TOKENS.md](DESIGN_TOKENS.md)); light and dark themes, following the system
  with a toggle.
- Release data (installer, size, SHA-256, `.sig`), the device list, the fan chip list
  and the user guide are resolved at build time. The published pages make no
  requests to other origins (Content-Security-Policy `default-src 'self'`).
- All copy is in `website/src/content/site.ts`, checked against the docs on `main`
  and against the owner rules (beta labels, no "confirmed/tested" device claims, D9).

## Screenshots

`docs/website/screenshots/<page>-<width>-<theme>.jpg` for every page at 375, 768 and
1440 px in light and dark (30 full-page captures). The theme variants shown for the
decision are in `docs/website/themes/`.

## Checks

| Check | Result |
|---|---|
| Layout at 375 / 768 / 1440 px, light and dark | No horizontal overflow, one `h1` per page, no console errors, no external requests (Playwright, 30 runs) |
| axe-core, WCAG 2.0/2.1 A + AA and best practices | 0 violations on all pages, both themes, 375 and 1440 px |
| Review (compliance, code, accessibility), each finding verified | 11 confirmed findings; 8 fixed here, 3 are outside this task's scope (open items) |

Lighthouse 12.3 (local production build, Edge headless):

| Page | Mode | Performance | Accessibility | Best practices | SEO | CLS | LCP |
|---|---|---|---|---|---|---|---|
| home | mobile | 99 | 100 | 100 | 100 | 0 | 2.2 s |
| home | desktop | 100 | 100 | 100 | 100 | 0 | 0.5 s |
| download | mobile | 99 | 100 | 100 | 100 | 0 | 2.1 s |
| download | desktop | 100 | 100 | 100 | 100 | 0 | 0.4 s |
| devices | mobile | 99 | 100 | 100 | 100 | 0 | 2.1 s |
| devices | desktop | 100 | 100 | 100 | 100 | 0 | 0.4 s |
| docs | mobile | 99 | 100 | 100 | 100 | 0 | 1.8 s |
| docs | desktop | 100 | 100 | 100 | 100 | 0 | 0.4 s |
| privacy | mobile | 99 | 100 | 100 | 100 | 0 | 2.0 s |
| privacy | desktop | 100 | 100 | 100 | 100 | 0 | 0.4 s |

## Open items (outside this task's edit scope or owner decisions)

1. `.github/workflows/website.yml`: pass `GITHUB_TOKEN` to the build step (the
   release fetch now fails the CI build instead of shipping a Download page without
   an installer), add `docs/USER_GUIDE.md`, `docs/openrgb-compatibility.json`,
   `docs/fan-coverage.json` and `docs/screenshots/**` to the path filter, and
   consider a `release: published` trigger so a new release updates the site.
   Optionally set `SITE_ORIGIN` if the site moves to its own domain.
2. `docs/PRIVACY.md` (F13/F14) and `docs/legal/ASSET_LIST.md` still say the website
   loads Google Fonts and calls `api.github.com` at runtime. Both are no longer true.
3. `docs/WEBSITE_CONTENT.md` is no longer read by the website; copy now lives in
   `website/src/content/site.ts`. The file should point there or be retired.
4. `website/src/data/hardware.json` and `scripts/website/build-hardware-db.mjs` are no
   longer used by the site (the Devices page uses `docs/openrgb-compatibility.json`).
5. The site uses the UI lead's refreshed screenshots (1600 × 1000, from
   `scripts/capture-screenshots.mjs`); they still carry the app's "Mock data" label.
   `docs/screenshots/README.md` still describes the older 1440 × 1000 capture. After
   the next refresh, run `node website/build/optimize-images.mjs`.
6. Owner decisions still open: D6 (licence of logo, icons and screenshots), Impressum
   (lawyer question), final legal texts (D5). The Privacy page states only facts about
   the website and links to the drafts.
7. Star count is not shown (it is 1); the build already fetches it if wanted later.
