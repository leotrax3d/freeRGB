# freeRGB website

The public website for freeRGB: Home, Download, Devices, Docs and Privacy. It is
built with Vite, React, Tailwind CSS and shadcn/ui (Radix primitives,
`components.json`). Design notes and tokens are in
[docs/website](../docs/website/).

```bash
npm ci
npm run dev        # http://localhost:5173
npm run build      # writes dist/
npm run preview
```

## How the build works

`npm run build` runs these steps:

1. `build/fetch-release.mjs` reads the latest release (assets, sizes, SHA-256)
   and the star count from the GitHub API and writes
   `src/data/release.generated.json` (ignored by Git). If GitHub cannot be
   reached, the previous file is kept, or the site builds without release data.
   Set `GITHUB_TOKEN` to avoid API rate limits.
2. `tsc -b` type-checks.
3. `vite build` builds the five pages (`index.html`, `download/`, `devices/`,
   `docs/`, `privacy/`). `build/site-data.ts` provides build-time modules:
   - `virtual:release`: the release data from step 1.
   - `virtual:devices`: the device list from `docs/openrgb-compatibility.json`.
   - `virtual:fans`: the fan controller chips from `docs/fan-coverage.json`.
   - `virtual:guide`: `docs/USER_GUIDE.md` rendered to HTML.
4. `vite build --ssr` and `build/prerender.mjs` write the server-rendered markup
   into each page's HTML. The browser then hydrates it.

The published pages make no requests to other origins. The production build sets
a Content-Security-Policy that allows no other origin (`default-src 'self'`).

## Content

All copy is in [`src/content/site.ts`](src/content/site.ts). Its header lists the
wording rules: only state what is on `main`, label untested hardware as beta, and
never call a device or board confirmed or tested.

## Screenshots

The app screenshots come from `docs/screenshots` (see its README for how to
capture them). After refreshing them, regenerate the AVIF/WebP variants:

```bash
npm install --no-save sharp
node build/optimize-images.mjs
```

## Website with the desktop demo

`npm run build:site` runs `../scripts/build-website.mjs`, which also builds the
desktop app's browser demo into `dist/demo/` and the admin panel into
`dist/admin/`. Set `BASE_PATH` (for example `/freeRGB/`) when the site is served
from a sub-path:

```bash
npm ci --prefix ../apps/desktop
BASE_PATH=/freeRGB/ npm run build:site
BASE_PATH=/freeRGB/ npm run preview
```

Details: [WEB_DEMO.md](../docs/WEB_DEMO.md).
