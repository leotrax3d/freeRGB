# Follow-up prompt — website "wow" interaction

Run after (or as step 6b of) [website-redesign.md](website-redesign.md).
Same worktree rules, same skills; this adds one signature interaction.

```text
You own the signature interaction of the freeRGB website. It must make people
say "wow" in the first five seconds and it must be *about freeRGB*: the visitor
is not watching an effect, they are controlling RGB lighting with their mouse,
keyboard or finger, the same way the app does.

## Concept: "Your cursor is the light"
The hero shows a real-looking LED scene drawn from actual device layouts in
this repo: a keyboard (use the A5 keyboard layout data), a fan ring and an
ARGB strip on a dark case outline. Every LED is a separate dot with bloom.
- Mouse: the pointer is a light source. LEDs near it pick up its hue with a
  physically plausible falloff; moving fast leaves a short trail that decays
  like the app's "reactive" effect. Hue follows pointer angle around the fan.
- Keyboard: pressing a real key lights the same key on the drawn keyboard and
  sends a ripple across all devices (the app's typing effect). Typing "rgb",
  "freergb" or the Konami-free word "rainbow" switches the scene effect.
- Touch: tap = ripple from that point; drag = paint; two fingers = rotate hue.
- Idle (no input for 4 s): the scene runs one of the real bundled effects.
- A tiny caption under the hero states the truth: "This is the freeRGB effect
  engine running in your browser" — and it must be true: compile the existing
  effect WASM modules (the 3 effects in the release catalog, effect API in
  crates/freergb-effect-*) for the web, or port their math 1:1 into a shared
  TS module with tests comparing frames against the Rust output.
- A "Try it on your setup" button leads to Download.

## Quality bar — what NOT to do
These are the cheap AI-site effects; none of them may appear:
cursor-follower blob/circle, mouse-trail sparkles, particle.js networks,
generic gradient mesh/aurora backgrounds, glass cards tilting on hover,
typewriter headline, scroll-jacking, random 3D blob, confetti, magnetic
buttons, "spotlight" radial gradient on cards. If an idea is something you
have seen on many landing pages, drop it.
The effect is judged by: meaning (it demonstrates the product), craft
(smooth 60 fps, exact timing, no jank, tasteful colour), restraint (one hero
moment, the rest of the page stays calm shadcn style).

## Engineering
- Rendering: one WebGL2 canvas (fallback Canvas2D), instanced points + bloom
  pass; no three.js unless the bundle stays small. Hero JS <= 60 KB gzip,
  lazy-load after first paint; LCP must not depend on it (static poster image
  from the new screenshots first).
- Input: Pointer Events (mouse+pen+touch unified), keyboard only while the
  hero is in view and no text field is focused; never block scrolling on
  touch (touch-action rules, passive listeners).
- Pause when off-screen (IntersectionObserver) or tab hidden; cap DPR at 2;
  measure with the Performance panel on a mid-range phone profile (4x CPU
  throttle) and keep frame time < 12 ms.
- prefers-reduced-motion: static lit scene, reacts only on click/tap, no
  trails. Provide a visible "Pause animation" toggle (WCAG 2.2.2).
- No flashing above 3 Hz anywhere (photosensitivity, WCAG 2.3.1); limit
  brightness swings. Canvas has aria-hidden plus a text description.
- Colour: hues from the design tokens; LEDs saturated, everything else
  neutral so the light is the only colour on the page.

## Skills
design:design-critique (judge 3 quick prototypes against the quality bar,
keep one), anthropic-skills:web-artifacts-builder (prototype the 3 variants
first and send the owner the links via t71 before building the final one),
run + anthropic-skills:chrome-browser (record desktop + mobile captures),
design:accessibility-review (motion, flashing, keyboard), engineering:
testing-strategy (frame-parity tests vs Rust effects), engineering:code-review.

## Done when
Owner picked a prototype, final version merged, Lighthouse still >= 95,
reduced-motion and touch verified in screenshots/screen recordings,
report to t71 with links.
```
