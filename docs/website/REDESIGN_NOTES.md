# Website redesign notes

Status: working notes for the `task/website-redesign` branch. Written 2026-11-08.

## 1. Critique of the current site (design:design-critique)

Screens reviewed: `website/` dev build at 1440 × 900 and 375 × 812, dark only.

### Overall impression

The site already looks calm and uses the desktop palette, which is a good base.
Its biggest problem is structure: it reads like a copy of the app shell (sidebar
plus inner "workspace" card) instead of a product page, so a first-time visitor
has no clear path from "what is this" to "can it drive my hardware" to "download".

### Usability

| Finding | Severity | Recommendation |
|---|---|---|
| Everything is one long page with 11 anchor targets. Download, device check and FAQ are far down. | 🔴 | Split into Home, Download, Devices, Docs, Privacy. Keep Home short and link out. |
| The desktop sidebar takes 224 px and repeats in-page anchors. On mobile it becomes a second scrolling tab row. | 🟡 | Use a slim top navigation (5 items) and a sheet menu on mobile. |
| The install area is a list of three links. There is no file size, no checksum and no word on SmartScreen or the `.sig` file. | 🔴 | Dedicated Download page: OS-detected primary button, asset table with size and SHA-256, verification steps, honest SmartScreen/beta notice. |
| Hardware search only shows `builtin` and `module` entries, with no filter and no "tested / not tested" marker. | 🟡 | Devices page with search, filter chips (category, support tier) and a visible "beta / not hardware-tested" label. |
| The site calls `api.github.com` on every page load. | 🟡 | Fetch release data at build time and ship static JSON. Zero third-party requests at runtime. |
| No hardware-damage or own-risk note anywhere on the site, though the app can lower fan speeds. | 🔴 | Short risk note on Download and in the Cooling feature, based on the README warning draft (docs/legal/SAFETY_LIABILITY_TEXTS.md c). |

### Visual hierarchy

- **What draws the eye first:** the headline, then the rainbow effect card. The
  card is the right idea (show light), but it is a saturated block, which feels
  closer to a gaming ad than a dev tool.
- **Reading flow:** headline → two buttons → card → tour tabs → long feature
  rows. The feature rows are text lists with small coloured icons; nothing shows
  the app itself until the tour.
- **Emphasis:** the beta status is only in the lead sentence. It should be a
  visible badge next to the version.

### Consistency

| Element | Issue | Recommendation |
|---|---|---|
| CSS | Two hand-written stylesheets (16 KB + 14 KB) override each other. Tailwind is configured but not wired up (no `@tailwind` directives, no PostCSS config), so shadcn components render with hand-written classes. | Wire up Tailwind and shadcn CSS variables; delete the hand-written sheets. |
| Tokens | `tailwind.config.js` refers to `--background`, `--primary` etc. that are never defined, and to a Manrope font that is never loaded. | Define the shadcn token set from the desktop values (see DESIGN_TOKENS.md). |
| Type | Body text sizes range from 9 px to 13 px. Many labels are 10–11 px. | Minimum 14 px for body copy, 12 px for captions only. |
| Feature icons | Six accent colours for icons; the app uses one accent plus semantic colours. | One accent; colour only where it carries meaning (the RGB glow). |

### Accessibility

- Contrast: `--disabled #858585` on `#212121` is about 4.4:1, used for 10–11 px
  text. Fails AA for small text.
- Touch targets: tag buttons and FAQ rows are fine; the 34 px icon links on mobile
  are just under 44 px.
- Text readability: too many 10–11 px sizes.
- Good: skip link, focus-visible ring, `prefers-reduced-motion` handling, `lang="en"`.

### What works well

- Calm black and graphite base, shared with the app.
- Honest copy: "Beta software", no superlatives.
- Self-hosted Inter, no cookies, no analytics.
- Live web demo opened in a separate tab without preloading.

## 2. Reference sites: what we take

Principles only. No text, logos, images or exact layouts are copied.

| Site | Take | Leave |
|---|---|---|
| cal.com | Open-source trust strip near the top: licence, GitHub link, star count, "runs locally, no account". Feature grid with short titles. FAQ before the footer. | Sign-up flows, pricing, social proof logos. |
| resend.com | Near-black, low-colour base. Strong type with large, tight headings and relaxed body. Thin 1 px borders on code and feature blocks. Long, even vertical rhythm. | Serif display face (does not match the app). |
| warp.dev | Desktop-app hero: a large framed product screenshot as the main visual. OS-aware download button with "other platforms" link. Feature sections each built around one real app screenshot. | Dense marketing banners. |
| linear.app | Dark background with soft radial glow and faint light beams behind the hero. Precise grid, small hover transitions, gradient hairline borders on key cards. | Heavy product UI walkthroughs. |

The RGB element: one slow, low-saturation glow behind the hero screenshot whose
hue cycles through the brand spectrum (red → amber → green → blue → violet) over
about 30 s. It is the only colour on the page besides the accent. Under
`prefers-reduced-motion` it stays still on a fixed hue.

## 3. Keep / change / take

**Keep**
- Black/graphite palette and Inter, aligned with `apps/desktop`.
- Beta labelling, factual tone, privacy stance.
- Live demo link (`/demo/`, new tab, no preload) and the `/admin/` bundle in `build:site`.
- `BASE_PATH` subpath support and Vite-imported assets.
- `docs/WEBSITE_CONTENT.md` as the content source.

**Change**
- Single long page → five pages (Home, Download, Devices, Docs, Privacy).
- App-shell sidebar → top navigation.
- Hand-written CSS → Tailwind + shadcn tokens.
- Runtime GitHub API call → build-time release snapshot.
- Rainbow block → subtle hue-cycling glow.
- Small 10–11 px text → 14 px minimum body.
- Add light theme (follows the system, with a toggle).

**Take from references**
- Trust strip (cal.com), calm type and borders (resend), screenshot-led hero and OS
  download button (warp), glow and gradient hairlines (linear).

## 4. Personas and section order (design:user-research, desk research only)

Sources: the product docs on main (README, USER_GUIDE, DEVICES, COOLING, INSTALL),
and common questions in public RGB/fan-control discussions. No interviews.

### P1 "Mixed-brand Windows gamer"
Windows 11, one PC with RGB parts from three or four vendors, each with its own
vendor tool running in the background.
- Does it see my devices? (→ device search)
- Can it replace the vendor apps, and must I uninstall them?
- Is it safe to install? Why does Windows warn me? (→ SmartScreen notice)
- Does it phone home, need an account or cost money?

### P2 "PC builder with fan curves"
Builds and tunes systems, cares about noise and temperatures, uses fan curves daily.
- Which fan controllers and boards work, and how well is that tested?
- Can I set curves from CPU/GPU/liquid temperature?
- What happens if the app crashes: do fans fall back safely?
- Can I verify the download (checksum, signature)?

### P3 "Linux user"
Runs a mainstream distro, used to OpenRGB, wants a native tool without Wine.
- Is there a Linux build, and which format (AppImage/deb/rpm)?
- What permissions does it need (udev rules, i2c, hwmon)?
- Is it open source, and under which licence?
- Can I check the source and build it myself?

### Resulting Home order
1. Hero: one-line value, beta badge, OS-aware download, live demo link, product screenshot with RGB glow. (all)
2. Trust strip: open source licence, runs locally, no account or telemetry, Windows + Linux. (P1, P3)
3. Device check teaser: search box that jumps to Devices. (P1, P2)
4. Features, each with a screenshot: Lighting, Cooling (with risk note), Monitoring, Profiles/Macros, Local API/plugins. (P1, P2)
5. Platforms: Windows and Linux details, link to Download. (P3)
6. FAQ: replacement for vendor apps, SmartScreen, safety, privacy, Linux permissions. (all)
7. Footer: licence, non-affiliation sentence, privacy, source.

## 5. Constraints found during discovery

- All legal texts in `docs/legal` and `docs/PRIVACY.md` are drafts marked "do not
  publish as is". The Privacy page states only facts about the website itself and
  links to the drafts; it does not present them as final.
- No Impressum is planned (docs/legal/OWNER_DECISIONS.md line 59; open lawyer question).
- Forbidden words: "safe", "certified", "tested for board X", "protects your
  hardware", "guaranteed". No donation or sponsor links.
- `docs/PRIVACY.md` (F13) and `docs/legal/ASSET_LIST.md` still mention Google Fonts.
  These files are outside this task's edit scope; listed as open items.
- The `/admin/` bundle under the same site talks to Appwrite. The Privacy page must say so.
- Screenshots in `docs/screenshots` date from 0.1.0-beta.2 (2026-10-24) and carry a
  "Mock data" label.
