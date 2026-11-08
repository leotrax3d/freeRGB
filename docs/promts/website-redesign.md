# Prompt — freeRGB website redesign

Paste the block below into a fresh Claude session (lead) in the repo root.

```text
You are the website lead for freeRGB. Redesign the project website in `website/`
(Vite + React + Tailwind + shadcn/ui, already set up). Work on branch
`task/website-redesign` in a worktree under E:/freergb-wt/website-redesign,
build output under E:/freergb-build. Never work in the main folder. Repo text
is English; talk to the owner in German.

## Goal
A world-class landing + download + docs site for a free, open-source RGB and
fan-control desktop app (Windows + Linux). It must feel like a premium dev
product, stay fully in shadcn/ui style (Radix primitives, Tailwind tokens,
`components.json`), and show RGB light tastefully, not like a gaming ad.

## Reference sites (owner's choice) — what to take from each
- cal.com: open-source trust signals (GitHub stars, licence, "self-host/own
  your data"), clear feature grid, honest pricing-free messaging, FAQ.
- resend.com: dark, elegant, low-colour base; typography quality; code/feature
  blocks with subtle borders; calm section rhythm.
- warp.dev: desktop-app hero with a big, animated product view; OS-aware
  download buttons; feature sections driven by real app screenshots.
- linear.app: dark background with soft glow and light beams; precise grid,
  micro-interactions, gradient borders. Use the glow as the "RGB" element
  (slowly cycling hue), respect prefers-reduced-motion.
Do not copy their text, logos, images or exact layouts; take principles only.

## Skills — use them in this order (load each with the Skill tool)
1. design:design-critique — critique the current website (screenshots via
   `run` / chrome-browser) and the four reference sites; output a short list
   "keep / change / take from reference" in docs/website/REDESIGN_NOTES.md.
2. design:user-research — 3 short personas (Windows gamer with mixed RGB
   brands, PC builder with fan curves, Linux user) and their top questions;
   use them to order the page sections. Desk research only, no interviews.
3. design:design-system — define tokens on top of the existing shadcn theme:
   colours (neutral dark base, one accent, RGB glow gradient), type scale,
   spacing, radius, shadows, motion. Align with the desktop app tokens in
   apps/desktop so site and app look like one product. Document in
   docs/website/DESIGN_TOKENS.md.
4. anthropic-skills:theme-factory — produce 2-3 variants of the token set
   (e.g. "graphite", "midnight", "aurora") as screenshots; stop and ask the
   owner (via t71) which one to use before step 6.
5. design:ux-copy + humanizer — write all copy: hero, features, download
   (incl. honest SmartScreen/beta notice), device support, FAQ, footer. Then
   run humanizer over every paragraph. No marketing superlatives, no
   unverifiable claims (only state what is on main; untested hardware is
   labelled "beta / not hardware-tested").
6. Implement (yourself or Codex workers, one per section, disjoint files):
   pages: Home, Download (OS detect, GitHub release assets, SHA256 + .sig
   explanation), Devices (searchable list generated from the module catalog /
   docs/OPENRGB_PORTING.md), Docs (render docs/USER_GUIDE.md), Privacy/Legal
   (existing texts in docs/legal, unchanged meaning). Use the fresh
   screenshots from the ui-lead (README image folder). Self-hosted fonts only,
   no external CDN, no trackers, no cookies.
   For interactive bits (device search, live colour demo) you may prototype
   with anthropic-skills:web-artifacts-builder first.
7. run + anthropic-skills:chrome-browser — start the dev server, capture
   screenshots at 375px, 768px, 1440px, light + dark, and review them
   against step 1 notes. Fix layout issues.
8. design:accessibility-review — WCAG 2.1 AA: contrast (also over glow),
   keyboard, focus rings, reduced motion, alt text, lang attribute.
9. legal:compliance-check — no third-party requests, privacy note correct,
   no personal data, licences of fonts/icons/images listed.
10. engineering:code-review — review the diff; fix findings.

## Rules
- Keep shadcn/ui; add components only via the shadcn CLI pattern into
  website/src/components/ui. No new UI framework.
- Performance: Lighthouse >= 95 on all four categories, no layout shift,
  images as AVIF/WebP with sizes.
- No hardware access, no UAC, no passwords, no edits outside website/,
  docs/website/, scripts/build-website.mjs and the screenshot folder.
- Gate: `npm ci && npm run build` in website/, `node scripts/build-website.mjs`
  ok, plus repo `cargo fmt --all --check` untouched. Merge to main after the
  local gate, delete the branch. Do not wait for CI.
- Report to t71: final screenshots paths, Lighthouse scores, open items.
```
