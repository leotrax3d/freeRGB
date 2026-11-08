# Follow-up prompt — website "wow" interaction

Run after (or as step 6b of) [website-redesign.md](website-redesign.md).
Same worktree rules, same skills; this adds the playful RGB layer.

```text
You own the playful RGB layer of the freeRGB website. Visitors should grin in
the first five seconds and want to keep moving the mouse, typing or touching.
Theme: RGB light. Playful game-like elements are welcome. What matters is that
every element is designed and built with real craft, not a copy-pasted
snippet.

## Ingredients (combine 3–5, prototype first, owner picks)
- Custom RGB cursor: e.g. a tiny LED, a light prism or a fan blade that
  spins with pointer speed; its colour cycles slowly and it casts real light
  on nearby UI (borders/edges pick up its hue). Hide it on touch devices and
  keep the system cursor for text fields and links (or a clear variant).
- Cursor as light source: LEDs of a drawn keyboard, fan ring and ARGB strip
  in the hero react to the pointer with falloff and a short decaying trail.
- Keyboard play: pressing a key lights the same key on the drawn keyboard
  and sends a ripple; hidden words ("rgb", "rainbow", "freergb") switch the
  effect or unlock a short colour show; a small "you found it" hint.
- Colour picker toy: drag across the hero to "paint" the LEDs; the chosen
  colour becomes the site accent (buttons, links, focus rings) until reload.
- Mini game (optional, small): e.g. "light all fans" or catch the colour,
  playable in 20 s, with score in the app's style; never blocks the page.
- Touch: tap = ripple, drag = paint, shake/tilt (DeviceMotion, opt-in) =
  colour burst. Haptics via vibrate() only after a user gesture.
- Sound: off by default; optional soft click per key press with a mute
  toggle.
- Section transitions: LED strip along the page edge that fills as you
  scroll (progress), in the current accent colour.
- Idle: after 4 s without input one of the real bundled freeRGB effects
  runs; if feasible use the actual effect math (WASM or 1:1 TS port with
  frame tests) and say so in a caption: "freeRGB effect engine, live".

## Quality bar
Any element is allowed (also custom cursors and trails) if it is done well:
- Own look: designed from the freeRGB token set and LED/fan motifs, not a
  library default. No unchanged snippets from codepen/particles.js/
  tutorials; no generic gradient blobs or confetti presets.
- Feel: 60 fps, eased motion with physical timing, no lag between input and
  light (< 1 frame), no jank on scroll.
- Restraint: one strong hero moment plus small touches elsewhere; content
  (download, features, docs) stays readable and calm shadcn style.
- Judge each prototype with design:design-critique: "Would this impress a
  designer at linear.app or warp.dev?" If not, iterate or drop it.

## Engineering
- One WebGL2 canvas (fallback Canvas2D) for LEDs + bloom; cursor as a
  separate lightweight layer. Playful layer JS <= 80 KB gzip, lazy-loaded
  after first paint; LCP from a static poster image.
- Pointer Events for mouse/pen/touch; keyboard only while hero is in view
  and no input is focused; never block scrolling (touch-action, passive).
- Pause when off-screen or tab hidden; cap DPR at 2; frame time < 12 ms on
  a 4x throttled mid-range phone profile.
- Accessibility: prefers-reduced-motion = static lit scene, system cursor,
  no trails; visible "Effects on/off" toggle remembered in localStorage;
  no flashing above 3 Hz (WCAG 2.3.1); canvas aria-hidden with text
  description; custom cursor never hides focus or hit targets.

## Skills
design:design-critique, anthropic-skills:web-artifacts-builder (build 3
combinations as prototypes, send links to the owner via t71, wait for the
pick), run + anthropic-skills:chrome-browser (desktop + mobile captures),
design:accessibility-review, engineering:testing-strategy,
engineering:code-review.

## Done when
Owner picked a combination, final version merged, Lighthouse still >= 95,
reduced-motion and touch verified, report to t71 with links.
```
