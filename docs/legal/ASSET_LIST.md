# Bundled asset inventory and provenance gaps

Repository evidence only, inspected 2026-11-08. This is a source-tree inventory,
including development/tool assets and examples; it does not assert that every file
is installed. Final binary and website payloads must be reconciled by release SHA
using [RELEASE_SCOPE.md](RELEASE_SCOPE.md). No external rights investigation was
performed. Author means credited author, not Git committer or catalog publisher.

Root `LICENSE-MIT` credits "The freeRGB contributors" (2026), alongside
`LICENSE-APACHE`. Those project texts are context, not proof that an unlabelled image,
font or imported design is covered. **Unknown** entries need attribution/permission
records or exclusion before release. The owner is one private natural person,
strictly non-commercial; catalog wording "freeRGB Team" does not identify the legal
publisher or establish authorship.

## Evidence keys (author / licence / provenance)

| Key | Credited author | Licence evidence | Repository provenance and open points |
|---|---|---|---|
| BRAND | Unknown | No asset-specific grant found | `assets/ASSETS.md` links Figma file `xoYLLKkvSQJNG1ViPVVfZW`, node 36-108: SVG exports, PNG rasterizations; loading animation recreates geometry/timing. Confirm designer, lettering rights and redistribution grant. |
| ICON | Unknown (derived branding) | No asset-specific grant found | `scripts/build-icons.py` derives `assets/icons/*` from `assets/brand/ring-dark.png`; `assets/ASSETS.md` describes sizes. Rights inherit unresolved BRAND questions. |
| LEGACY | Unknown | No asset-specific grant found | `assets/ASSETS.md` calls root artwork historical; original creator/source and grant unrecorded. |
| COPY | Unknown | No asset-specific grant found | `logo.png` is described as a compatibility copy in `assets/ASSETS.md`; Tauri icon origins are not documented there. Verify actual derivation/hashes. |
| DOC | Unknown | No file-specific grant found; project licence context only | Repository screenshots/progress artwork; verify creator, capture inputs and any depicted third-party material. ProductTour imports lighting/layout/cooling screenshots. |
| EFFECT | Individual author unknown; catalog publisher `leotrax3d` / `freeRGB Team` | No per-effect licence header found in guest sources; project licence context only | `scripts/build-effects.py`: first three guests use `effects/examples/effect.rs`; 25 collection guests use `effects/collection/effect.rs` plus `effects/collection.json`; catalogs record sizes/SHA-256. Rebuild/hash correspondence still needs checking. Development output is distinct from bundled packages. |
| EXAMPLE | Individual author unknown | Project licence context only; no separate attribution established here | In-tree example/fixture source or JSON, not an external attribution record; inspect imported content before distribution. |
| TOOL | Unknown | No asset-specific grant established | `skills/shadcn/assets/` tool/showcase material; origin and redistribution terms need tracing. Not presumed runtime payload. |
| META | Not established separately | Project licence context only | Asset documentation/metadata; content does not itself establish a rights chain. |

## Tracked files (each path listed)

The evidence key applies the author/licence/provenance columns above to each file.

| File | Evidence key |
|---|---|
| `apps/desktop/src-tauri/icons/128x128.png` | COPY |
| `apps/desktop/src-tauri/icons/32x32.png` | COPY |
| `apps/desktop/src-tauri/icons/icon.ico` | COPY |
| `apps/desktop/src-tauri/icons/icon.png` | COPY |
| `assets/ASSETS.md` | META |
| `assets/badges/code-lines.svg` | DOC |
| `assets/brand/github-avatar.png` | BRAND |
| `assets/brand/github-avatar.svg` | BRAND |
| `assets/brand/github-banner.png` | BRAND |
| `assets/brand/github-banner.svg` | BRAND |
| `assets/brand/github-social-preview.png` | BRAND |
| `assets/brand/github-social-preview.svg` | BRAND |
| `assets/brand/loading-ring-monogram.svg` | BRAND |
| `assets/brand/monogram-dark.png` | BRAND |
| `assets/brand/monogram-dark.svg` | BRAND |
| `assets/brand/monogram-light.png` | BRAND |
| `assets/brand/monogram-light.svg` | BRAND |
| `assets/brand/monogram-tray-dark.png` | BRAND |
| `assets/brand/monogram-tray-dark.svg` | BRAND |
| `assets/brand/monogram-tray-white.png` | BRAND |
| `assets/brand/monogram-tray-white.svg` | BRAND |
| `assets/brand/ring-dark.png` | BRAND |
| `assets/brand/ring-dark.svg` | BRAND |
| `assets/brand/ring-light.png` | BRAND |
| `assets/brand/ring-light.svg` | BRAND |
| `assets/brand/ring-tray-dark.png` | BRAND |
| `assets/brand/ring-tray-dark.svg` | BRAND |
| `assets/brand/ring-tray-white.png` | BRAND |
| `assets/brand/ring-tray-white.svg` | BRAND |
| `assets/brand/wordmark-dark.png` | BRAND |
| `assets/brand/wordmark-dark.svg` | BRAND |
| `assets/brand/wordmark-light.png` | BRAND |
| `assets/brand/wordmark-light.svg` | BRAND |
| `assets/icons/freergb-128.png` | ICON |
| `assets/icons/freergb-16.png` | ICON |
| `assets/icons/freergb-24.png` | ICON |
| `assets/icons/freergb-256.png` | ICON |
| `assets/icons/freergb-32.png` | ICON |
| `assets/icons/freergb-48.png` | ICON |
| `assets/icons/freergb-64.png` | ICON |
| `assets/icons/freergb.ico` | ICON |
| `assets/icons/freergb.png` | ICON |
| `assets/light.png` | LEGACY |
| `assets/light.svg` | LEGACY |
| `assets/light_no-gradient.svg` | LEGACY |
| `assets/monochrome.png` | LEGACY |
| `assets/monochrome.svg` | LEGACY |
| `assets/refined.png` | LEGACY |
| `assets/refined.svg` | LEGACY |
| `crates/freergb-effect-host/examples/benchmark.rs` | EXAMPLE |
| `crates/freergb-effect-host/examples/collection_preview.rs` | EXAMPLE |
| `crates/freergb-gpu-adlx/examples/adlx-discover.rs` | EXAMPLE |
| `crates/freergb-storage-temp/examples/storage-temps.rs` | EXAMPLE |
| `docs/assets/project-progress.svg` | DOC |
| `docs/screenshots/cooling-setup-curves.jpg` | DOC |
| `docs/screenshots/cooling-setup-detect.jpg` | DOC |
| `docs/screenshots/cooling-setup-identify.jpg` | DOC |
| `docs/screenshots/cooling.png` | DOC |
| `docs/screenshots/devices.png` | DOC |
| `docs/screenshots/layout.png` | DOC |
| `docs/screenshots/lighting.png` | DOC |
| `docs/screenshots/macros.png` | DOC |
| `docs/screenshots/marketplace.png` | DOC |
| `docs/screenshots/monitoring.png` | DOC |
| `docs/screenshots/profiles.png` | DOC |
| `docs/screenshots/settings.png` | DOC |
| `effects/catalog.json` | EFFECT |
| `effects/collection.json` | EFFECT |
| `effects/collection/effect.rs` | EFFECT |
| `effects/development/atemlicht-1.0.0.wasm` | EFFECT |
| `effects/development/aurora-1.0.0.wasm` | EFFECT |
| `effects/development/catalog.json` | EFFECT |
| `effects/development/comet-1.0.0.wasm` | EFFECT |
| `effects/development/cyberpunk-1.0.0.wasm` | EFFECT |
| `effects/development/farbfluss-1.0.0.wasm` | EFFECT |
| `effects/development/feuer-1.0.0.wasm` | EFFECT |
| `effects/development/feuerwerk-1.0.0.wasm` | EFFECT |
| `effects/development/galaxie-1.0.0.wasm` | EFFECT |
| `effects/development/gewitter-1.0.0.wasm` | EFFECT |
| `effects/development/gluehwuermchen-1.0.0.wasm` | EFFECT |
| `effects/development/herzschlag-1.0.0.wasm` | EFFECT |
| `effects/development/kaleidoskop-1.0.0.wasm` | EFFECT |
| `effects/development/konfetti-1.0.0.wasm` | EFFECT |
| `effects/development/lava-1.0.0.wasm` | EFFECT |
| `effects/development/matrixregen-1.0.0.wasm` | EFFECT |
| `effects/development/meteorschauer-1.0.0.wasm` | EFFECT |
| `effects/development/neonroehren-1.0.0.wasm` | EFFECT |
| `effects/development/ozean-1.0.0.wasm` | EFFECT |
| `effects/development/pendel-1.0.0.wasm` | EFFECT |
| `effects/development/plasma-1.0.0.wasm` | EFFECT |
| `effects/development/polarlicht-1.0.0.wasm` | EFFECT |
| `effects/development/prism-1.0.0.wasm` | EFFECT |
| `effects/development/radar-1.0.0.wasm` | EFFECT |
| `effects/development/reaktionsdiffusion-1.0.0.wasm` | EFFECT |
| `effects/development/regentropfen-1.0.0.wasm` | EFFECT |
| `effects/development/schachbrett-1.0.0.wasm` | EFFECT |
| `effects/development/schwarzes-loch-1.0.0.wasm` | EFFECT |
| `effects/development/sternenhimmel-1.0.0.wasm` | EFFECT |
| `effects/examples/effect.rs` | EFFECT |
| `effects/packages/aurora-1.0.0.wasm` | EFFECT |
| `effects/packages/comet-1.0.0.wasm` | EFFECT |
| `effects/packages/prism-1.0.0.wasm` | EFFECT |
| `examples/plugins/virtual-linux.json` | EXAMPLE |
| `examples/plugins/virtual-windows.json` | EXAMPLE |
| `logo.png` | COPY |
| `skills/shadcn/assets/Notizbuch öffnen.onetoc2` | TOOL |
| `skills/shadcn/assets/shadcn-small.png` | TOOL |
| `skills/shadcn/assets/shadcn.png` | TOOL |
| `skills/shadcn/assets/showcase/Notizbuch öffnen.onetoc2` | TOOL |
| `skills/shadcn/assets/showcase/color-pair.tsx` | TOOL |
| `skills/shadcn/assets/showcase/preview-states.css` | TOOL |
| `skills/shadcn/assets/showcase/state-matrix.tsx` | TOOL |

## Dependency-provided fonts and icons; website surfaces

No tracked font binary was found in the file inventory. Installed/build-generated
files are a separate payload inventory, not evidence of absence from binaries.

| Asset family | Author | Licence/provenance shown by repo | Open points |
|---|---|---|---|
| `apps/desktop`: `@fontsource-variable/inter` 5.3.0 | Not credited in lock metadata | Lockfile declares `OFL-1.1`; registry source `https://registry.npmjs.org/@fontsource-variable/inter/-/inter-5.3.0.tgz`, integrity in lockfile | Inspect actual package copyright/licence/attribution and emitted subset, retain required texts; enumerate/hash output files at release. |
| `apps/desktop`: `lucide-react` 1.60.0 | Not credited in lock metadata | Lockfile declares `ISC`; registry source `https://registry.npmjs.org/lucide-react/-/lucide-react-1.60.0.tgz`, integrity in lockfile | Inspect actual package copyright/licence/attribution and emitted subset, retain required texts; enumerate/hash output files at release. |
| `website`: `lucide-react` 0.468.0 | Not credited in lock metadata | Lockfile declares `ISC`; registry source `https://registry.npmjs.org/lucide-react/-/lucide-react-0.468.0.tgz`, integrity in lockfile | Inspect actual package copyright/licence/attribution and emitted subset, retain required texts; enumerate/hash output files at release. |
| Website remote Inter | Unknown from repository | `website/src/index.css` imports Google Fonts CSS (Inter 400/500/600/700); remote font bytes/licence not stored here | Capture exact served font provenance and terms or replace remote dependency; remote requests also need privacy review. |
| System fallback fonts | Platform suppliers; individual authors not established | CSS names system/monospace fallbacks and badge SVG names Verdana/Geneva/DejaVu Sans | References are not bundled font files; verify any rasterization/embedding inputs. |
| Website images | See BRAND, ICON, DOC above | `website/src/App.tsx` imports wordmark-dark.svg and freergb-32.png; ProductTour imports docs/screenshots/{lighting,layout,cooling}.png | No separate tracked website image directory found. Build may emit renamed copies; reconcile paths/hashes. |
| Inline UI artwork/effect previews | Individual creators uncredited | `apps/desktop/src/components/brand.tsx`, website components and effect playground render shapes/previews from code; Lucide imports render SVG icons | Source falls under project/dependency context; inspect final emitted artwork and retain dependency notices. |

## Completion work

- Obtain designer/export ownership and redistribution records for Figma and historical artwork, Tauri icons and outlined lettering; record fonts used to create outlines.
- Identify screenshot creators and depicted material; trace shadcn tool images and unusual tool files before including them in a source distribution.
- Confirm effect source-to-WASM and catalog hash correspondence; record author/licence evidence rather than substituting publisher labels.
- At the release SHA enumerate extracted installer, archives and website output, including dependency-generated icon/font files and externally supplied prerequisite payload. Add hashes, exact versions, upstream copyright texts and required notices. See SBOM limitations for dependencies beyond this media inventory.
