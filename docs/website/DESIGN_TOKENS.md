# Website design tokens

The website uses the shadcn/ui token model (HSL channels in CSS variables, consumed by
`website/tailwind.config.js`). Values are taken from the desktop app
(`apps/desktop/src/styles/globals.css`) so the site and the app read as one product.
Where the website needs something the app does not have (a light theme, larger
type, the hero glow), it is added here and marked **site only**.

Source of truth in code: `website/src/index.css` (`:root` = light, `.dark` = dark).
The site defaults to the system colour scheme and offers a toggle.

## Colour

### Neutral base

| Token | Dark | Light | Desktop equivalent |
|---|---|---|---|
| `--background` | `#000000` | `#fafafa` | `bg` / brand light tile |
| `--foreground` | `#ececec` | `#171717` | `text` |
| `--card` | `#171717` | `#ffffff` | `surface-sunken` |
| `--card-foreground` | `#ececec` | `#171717` | `text` |
| `--popover` | `#212121` | `#ffffff` | `surface` |
| `--secondary` | `#303030` | `#f0f0f0` | `surface-raised` |
| `--muted` | `#212121` | `#f0f0f0` | `surface` |
| `--muted-foreground` | `#b4b4b4` | `#525252` | `text-muted` |
| `--subtle-foreground` (site only) | `#a0a0a0` | `#616161` | `text-subtle` (raised for AA) |
| `--border` | `#262626` | `#e5e5e5` | `border` (white 8 %) |
| `--border-strong` (site only) | `#3a3a3a` | `#d4d4d4` | `border-strong` (white 16 %) |
| `--input` | `#3a3a3a` | `#d4d4d4` | `border-strong` |
| `--ring` | `#3b82f6` | `#2563eb` | `info` focus ring |

### Actions and accent

The app rule is kept: the primary button is light-on-dark (dark-on-light in the
light theme), and the green accent marks state, not button fills, because white on
`#10a37f` is only 3.2:1.

| Token | Dark | Light | Use |
|---|---|---|---|
| `--primary` | `#ececec` | `#171717` | Primary button fill |
| `--primary-foreground` | `#000000` | `#fafafa` | Primary button text |
| `--brand` | `#10a37f` | `#10a37f` | Dots, active markers, check icons, the "G" of the logo |
| `--brand-foreground` (text) | `#3ccf9f` | `#0b7a5f` | Accent text and links on the base (≥ 4.5:1) |
| `--warning` | `#f59e0b` | `#b45309` | Beta / not hardware-tested labels |
| `--destructive` | `#f87171` | `#dc2626` | Risk notes |

### RGB glow (site only)

The spectrum is the app's `SPECTRUM` (`apps/desktop/src/lib/api/effects.ts:43`):

| Stop | Hex |
|---|---|
| red | `#ef4444` |
| amber | `#f59e0b` |
| green | `#10a37f` |
| blue | `#3b82f6` |
| violet | `#a855f7` |

- `--glow-opacity`: dark `0.32`, light `0.18`.
- `--glow-blur`: `96px`.
- The glow is two or three radial gradients behind the hero screenshot. A `hue-rotate`
  animation (`--glow-duration: 40s`, linear, infinite) moves it through the spectrum.
- Gradient hairline: 1 px border made from the same stops at 40 % opacity, used on
  the hero frame and the primary download card only.
- Text never sits directly on the glow. Every glow area has a solid surface
  (`--card` or the screenshot) between the glow and any text, so contrast is
  measured against the surface, not the glow.
- `prefers-reduced-motion: reduce` → no animation, fixed hue (green-blue).

## Typography

Font: Inter Variable, self-hosted (`@fontsource-variable/inter`, OFL-1.1), with
features `cv11`, `ss01` as in the app. Mono: `ui-monospace, "Cascadia Code",
"SF Mono", Consolas, monospace`.

The app uses 12–28 px (dense UI). The site uses a larger reading scale built on
the same weights (400/500/600).

| Token | Size / line height | Weight | Tracking | Use |
|---|---|---|---|---|
| `display` | clamp(40px, 6vw, 68px) / 1.04 | 600 | -0.045em | Hero headline |
| `h1` | clamp(32px, 4vw, 48px) / 1.1 | 600 | -0.035em | Page titles |
| `h2` | clamp(26px, 3vw, 36px) / 1.15 | 600 | -0.03em | Section titles |
| `h3` | 20px / 28px | 600 | -0.015em | Card titles |
| `lead` | 18px / 30px | 400 | 0 | Intro paragraphs |
| `body` | 16px / 26px | 400 | 0 | Running text |
| `small` | 14px / 22px | 400 | 0 | Secondary text, table cells |
| `caption` | 12px / 16px | 500 | 0.01em | Labels, badges (never paragraphs) |
| `code` | 13px / 20px mono | 400 | 0 | Hashes, commands |

## Spacing and layout

- 4 px grid (4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 96, 128), same as the app.
- Container: max 1200 px; side gutter 16 px (< 640), 24 px (< 1024), 32 px.
- Section rhythm: 96 px vertical on desktop, 64 px on mobile.
- Grid: 12 columns, 24 px gap (desktop); single column below 768 px.

## Radius

| Token | Value | Desktop equivalent | Use |
|---|---|---|---|
| `--radius` (`rounded-lg`) | 10px | `control` | Buttons, inputs |
| `rounded-md` | 8px | — | Small controls |
| `rounded-sm` | 6px | `chip` | Badges, chips |
| `rounded-card` | 14px | `card` | Cards |
| `rounded-panel` | 20px | `panel` | Hero frame, large panels |

## Shadows

Elevation comes mostly from surface colour, as in the app.

| Token | Value | Use |
|---|---|---|
| `shadow-popover` | `0 8px 24px rgb(0 0 0 / .5), 0 0 0 1px rgb(255 255 255 / .08)` | Menus, sheet (same as app) |
| `shadow-frame` | `0 40px 80px -24px rgb(0 0 0 / .55)` (light: `/ .18`) | Screenshot frames |

## Motion

| Token | Value | Use |
|---|---|---|
| `--ease-out` | `cubic-bezier(0.2, 0, 0, 1)` | All transitions (same as app) |
| `--duration-fast` | 120ms | Hover, press |
| `--duration-base` | 180ms | Enter, accordion |
| `--duration-slow` | 240ms | Sheet, panels |
| `--glow-duration` | 40s | Hero hue cycle (site only) |

Rules: no scroll-triggered layout animation (keeps CLS at 0); no motion on content
that the user has to read; `prefers-reduced-motion` turns off all animation and
transitions.

## Theme variants

Three variants were built on this token set for the owner to choose from
(`docs/website/themes/`):

- **graphite**: the values above, closest to the app.
- **midnight**: blue-tinted near-black base, slate cards.
- **aurora**: graphite base with a stronger, wider glow and gradient hairlines.

The chosen variant is recorded here once decided.
