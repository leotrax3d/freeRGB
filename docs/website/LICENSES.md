# Website: third-party material and licences

Everything below ships in the built website (`website/dist`). Nothing is loaded
from another origin at runtime.

## Fonts

| Font | Package | Licence |
|---|---|---|
| Inter (variable) | `@fontsource-variable/inter` 5.3.0, self-hosted WOFF2 | SIL Open Font License 1.1 |

The licence text is in `packaging/licenses/Inter-OFL-1.1.txt` and in the package
(`website/node_modules/@fontsource-variable/inter/LICENSE`).

## Icons

| Icons | Package | Licence |
|---|---|---|
| Lucide | `lucide-react` 0.468.0, bundled as inline SVG | ISC (parts derived from Feather, MIT) |

## Images

| Image | Source | Licence / status |
|---|---|---|
| App screenshots (`website/src/assets/screens/*.avif, *.webp`) | Converted from `docs/screenshots/*.png` by `website/build/optimize-images.mjs` | Project material; author/licence not yet recorded (see `docs/legal/ASSET_LIST.md`, decision D6) |
| Logo ring (`assets/brand/ring-dark.svg`, `website/public/favicon.svg`) | `assets/brand` | Project brand; licence not yet recorded (D6) |
| Favicon PNG, touch icon (`website/public/favicon-32.png`, `apple-touch-icon.png`) | `assets/icons` | Project brand; licence not yet recorded (D6) |
| Social preview (`website/public/og-image.png`) | Resized from `assets/brand/github-social-preview.png` | Project brand; licence not yet recorded (D6) |

## Runtime JavaScript dependencies

All are bundled into the site's own JavaScript files.

| Package | Licence |
|---|---|
| react, react-dom, scheduler | MIT |
| @radix-ui/* (accordion, dialog, tabs, tooltip, slot, separator and their internals) | MIT |
| @floating-ui/* | MIT |
| class-variance-authority | Apache-2.0 |
| clsx, tailwind-merge | MIT |
| aria-hidden, react-remove-scroll, react-remove-scroll-bar, react-style-singleton, detect-node-es, get-nonce, use-callback-ref, use-sidecar | MIT |
| tslib | 0BSD |

## Build-time only (not shipped)

Vite, TypeScript, Tailwind CSS, PostCSS, Autoprefixer, tailwindcss-animate and
marked (MIT) run during the build. `marked` renders `docs/USER_GUIDE.md` to HTML
at build time; no Markdown library ships to the browser.

## Data shown on the site

| Data | Source |
|---|---|
| Release name, assets, sizes, SHA-256 | GitHub REST API, fetched by `website/build/fetch-release.mjs` at build time |
| Device list | `docs/openrgb-compatibility.json` (generated from the OpenRGB source tree, GPL-2.0-or-later project; only device names and USB IDs are used) |
| Fan controller chips | `docs/fan-coverage.json` (derived from LibreHardwareMonitor, MPL-2.0; only chip names, vendors and board names are used) |
| User guide | `docs/USER_GUIDE.md` |

`THIRD_PARTY_NOTICES.md` at the repository root covers the desktop app only. The
website's dependencies are listed here.
