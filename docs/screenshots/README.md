# README gallery

Captured on 2026-10-24 from freeRGB `0.1.0-beta.2`, using the actual React
frontend's built-in browser mock at 1440 × 1000, Chromium, dark theme, English.
All nine main views are included. No physical device, native driver, production
catalog or sensor reading was used. The visible “Mock data” label is retained.
Screenshots are unretouched PNG captures; they illustrate UI rather than acceptance.

## Refresh

Start `npm run dev -- --host 127.0.0.1` in `apps/desktop`, then from the repository
root run:

```bash
node scripts/capture-readme-gallery.mjs
```

The script requires Playwright available to Node. If installed outside the checkout,
set `PLAYWRIGHT_MODULE` to its `index.mjs` path. Set `CHROMIUM_PATH` to use a system
Chromium instead of a Playwright-managed browser. No new application dependency
is required. Example:

```bash
PLAYWRIGHT_MODULE=/path/to/node_modules/playwright/index.mjs \
CHROMIUM_PATH=/usr/bin/chromium node scripts/capture-readme-gallery.mjs
```

Only the local development origin is allowed. The script verifies mock transport,
skips hardware setup, captures each page after its heading/fonts are ready and
fails on JavaScript page errors. It never enables real fan control. Inspect the
images and update this capture date/version when refreshing the gallery.

## Guided fan setup

Captured on 2026-11-05 from the built-in browser mock (Chromium, dark theme,
800 px wide pane). Nothing was written to hardware; chip names come from the
mock inventory, RPM rises and calibration ranges are simulated.

| Step | Image |
|---|---|
| Detect chips and support tiers | [cooling-setup-detect.jpg](cooling-setup-detect.jpg) |
| Identify, name and auto-pair the tachometer | [cooling-setup-identify.jpg](cooling-setup-identify.jpg) |
| Proposed curve per confirmed fan | [cooling-setup-curves.jpg](cooling-setup-curves.jpg) |
