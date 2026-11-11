// Scroll motion for the website: Lenis smooth scroll + GSAP ScrollTrigger.
// Driven by data attributes in the prerendered markup, so nothing is hidden in the
// static HTML. public/theme-init.js adds `motion-pending` to <html> (JS on); CSS hides
// reveal targets only under that class until initMotion() runs.
//
// Style: few large scroll-linked moments (scrub) plus quick, short text reveals.
//   data-hero          words and lines rise quickly on load ([data-word] = split words)
//   data-hero-shot     tilted back and smaller, lies flat as you scroll
//   data-reveal        children reveal quickly on enter
//   data-batch         cards rise with the scroll, staggered
//   data-feature       text reveals; screenshot ([data-scrub]) opens from a smaller window
//   data-reveal-scale  grows into place with the scroll
//   data-header        gets data-scrolled after 8 px
//   data-progress      scaleX scroll progress
// Reduced motion: short opacity fades only, native scrolling, no movement.
import gsap from "gsap"
import { ScrollTrigger } from "gsap/ScrollTrigger"
import Lenis from "lenis"

const HEADER_OFFSET = 80
let lenis: Lenis | null = null
let ctx: gsap.Context | null = null

export const prefersReducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches

function release() {
  document.documentElement.classList.remove("motion-pending")
}

const done = { clearProps: "transform,willChange" }
const $$ = (s: string, root: ParentNode = document) => Array.from(root.querySelectorAll<HTMLElement>(s))

export function scrollToTarget(target: string | HTMLElement) {
  if (lenis) lenis.scrollTo(target, { offset: -HEADER_OFFSET })
  else (typeof target === "string" ? document.querySelector(target) : target)?.scrollIntoView()
}

/** Scroll-linked parallax for one element (768 px and up). Returns a cleanup function. */
export function parallax(el: HTMLElement, distance = 30) {
  if (prefersReducedMotion()) return () => {}
  gsap.registerPlugin(ScrollTrigger)
  const mm = gsap.matchMedia()
  mm.add("(min-width: 768px)", () => {
    gsap.fromTo(el, { y: distance }, { y: -distance, ease: "none", scrollTrigger: { trigger: el, start: "top bottom", end: "bottom top", scrub: true } })
  })
  return () => mm.revert()
}

/**
 * Hover tilt towards the pointer (fine pointers only). Uses the CSS `rotate` property,
 * which composes with GSAP's `transform`, so it never fights the scroll animations.
 */
function tilt(el: HTMLElement, max: number) {
  if (!window.matchMedia("(hover: hover) and (pointer: fine)").matches) return
  el.parentElement?.style.setProperty("perspective", "1200px")
  el.style.transition = "rotate 240ms cubic-bezier(0.2, 0, 0, 1)"
  el.addEventListener("pointermove", (e) => {
    const r = el.getBoundingClientRect()
    const dx = (e.clientX - r.left) / r.width - 0.5, dy = (e.clientY - r.top) / r.height - 0.5
    const len = Math.hypot(dx, dy) || 1
    el.style.rotate = `${-dy / len} ${dx / len} 0 ${Math.min(1, len * 2) * max}deg`
  })
  el.addEventListener("pointerleave", () => (el.style.rotate = ""))
}

function reveal(targets: Element[], trigger: Element, calm: boolean) {
  if (!targets.length) return
  gsap.fromTo(targets, calm ? { opacity: 0 } : { opacity: 0, y: 14 }, {
    opacity: 1, y: 0, duration: calm ? 0.4 : 0.55, ease: "power2.out", stagger: 0.05,
    scrollTrigger: { trigger, start: "top 92%", once: true }, ...done,
  })
}

/** Resolves a theme token ("161 60% 52%") to a colour GSAP can interpolate. */
const token = (name: string) => `hsl(${getComputedStyle(document.documentElement).getPropertyValue(name).trim().split(/\s+/).join(", ")})`

const scrub = (trigger: Element, start: string, end: string) => ({ trigger, start, end, scrub: true })

function build(calm: boolean) {
  for (const hero of $$("[data-hero]")) {
    const items = Array.from(hero.children).flatMap((c) => (c.querySelector("[data-word]") ? $$("[data-word]", c) : [c]))
    gsap.fromTo(items, calm ? { opacity: 0 } : { opacity: 0, yPercent: 40 }, { opacity: 1, yPercent: 0, duration: calm ? 0.4 : 0.6, ease: "expo.out", stagger: 0.035, ...done })
  }
  for (const shot of $$("[data-hero-shot]")) {
    gsap.fromTo(shot, { opacity: 0 }, { opacity: 1, duration: 0.5, delay: 0.15, ease: "power2.out" })
    if (calm) continue
    shot.parentElement?.style.setProperty("perspective", "1600px")
    gsap.fromTo(shot, { rotateX: 22, scale: 0.88, y: -20, transformOrigin: "50% 0%" }, { rotateX: 0, scale: 1, y: 0, ease: "none", scrollTrigger: scrub(shot, "top 95%", "top 25%") })
  }
  for (const el of $$("[data-reveal]")) reveal(Array.from(el.children), el, calm)
  for (const el of $$("[data-reveal-scale]")) {
    if (calm) reveal([el], el, true)
    else gsap.fromTo(el, { scale: 0.86 }, { scale: 1, ease: "none", scrollTrigger: scrub(el, "top bottom", "top 55%") })
  }
  const mm = gsap.matchMedia()
  for (const el of $$("[data-feature]")) {
    const [text, media] = Array.from(el.children) as HTMLElement[]
    const lines = text ? (Array.from(text.children) as HTMLElement[]) : []
    const frame = media?.querySelector<HTMLElement>("[data-scrub]")
    if (calm || !frame) { reveal(lines, el, calm); if (frame) reveal([frame], el, true); continue }
    const side = el.hasAttribute("data-flipped") ? -1 : 1
    // One feature at a time: the section holds still while its text builds line by line,
    // then the screenshot drives in from its side. Only then the page moves on.
    mm.add({ wide: "(min-width: 1024px)", narrow: "(max-width: 1023px)" }, (c) => {
      const wide = c.conditions?.wide
      const tl = gsap.timeline({
        defaults: { ease: "power2.out" },
        // Builds while the section enters and is complete by the time it reaches the
        // middle of the screen, so nothing in view is ever empty.
        scrollTrigger: { trigger: el, start: "top 90%", end: wide ? "center 55%" : "top 30%", scrub: true },
      })
      tl.fromTo(lines, { opacity: 0, y: 40 }, { opacity: 1, y: 0, stagger: 0.25, duration: 0.6 })
        .fromTo(frame, { xPercent: (wide ? 70 : 30) * side, rotate: 6 * side, opacity: 0 }, { xPercent: 0, rotate: 0, opacity: 1, duration: 1 }, "-=0.2")
        .to({}, { duration: 0.4 })
    })
    tilt(frame, 5)
  }
  for (const list of $$("[data-batch]")) {
    const items = Array.from(list.children) as HTMLElement[]
    if (calm) { reveal(items, list, true); continue }
    // Cards tilt up from the page. Lists whose cards share borders (gap-px) tilt as one
    // piece, so no gaps open between them mid-way. Opacity is full early, so the
    // in-between state never shows half-transparent, overlapping cards.
    const joined = list.classList.contains("gap-px")
    const targets = joined ? [list] : items
    list.parentElement!.style.perspective = "1400px"
    if (!joined) list.style.perspective = "1400px"
    const st = { trigger: list, start: "top bottom", end: "top 60%", scrub: true }
    gsap.fromTo(targets, { y: 60, rotateX: 28, transformOrigin: "50% 100%" }, { y: 0, rotateX: 0, ease: "power1.out", stagger: 0.1, scrollTrigger: st })
    gsap.fromTo(targets, { opacity: 0 }, { opacity: 1, ease: "none", stagger: 0.1, scrollTrigger: { ...st, end: "top 85%" } })
    if (!joined) for (const card of items) tilt(card, 6)
  }
  // Footprint numbers: the headline moment. Cards punch in one after another, each value
  // races up from 0 with a motion blur that resolves as it lands, and a brand line sweeps
  // under it. Final values are in the static HTML, so nothing depends on this running.
  for (const grid of $$("[data-stats]")) {
    const cards = $$("[data-stat]", grid)
    if (calm) { reveal(cards, grid, true); continue }
    const tl = gsap.timeline({ scrollTrigger: { trigger: grid, start: "top 85%", once: true } })
    tl.fromTo(cards, { opacity: 0, y: 70, scale: 0.85, rotateX: 25, transformOrigin: "50% 100%" }, { opacity: 1, y: 0, scale: 1, rotateX: 0, duration: 0.9, ease: "back.out(1.6)", stagger: 0.09 })
    cards.forEach((card, i) => {
      const at = i * 0.09 + 0.15
      for (const el of $$("[data-count]", card)) {
        const end = Number(el.dataset.count), decimals = Number(el.dataset.decimals) || 0, state = { v: 0 }
        tl.fromTo(state, { v: 0 }, { v: end, duration: 1.6, ease: "expo.out", onUpdate: () => { el.textContent = state.v.toFixed(decimals) } }, at)
          .fromTo(el, { filter: "blur(8px)", opacity: 0.4 }, { filter: "blur(0px)", opacity: 1, duration: 1.1, ease: "power3.out", clearProps: "filter" }, at)
      }
      const value = card.querySelector("dd > span")
      if (value) tl.fromTo(value, { color: token("--brand-foreground") }, { color: token("--foreground"), duration: 1.4, ease: "power2.inOut", clearProps: "color" }, at + 0.3)
    })
    grid.style.perspective = "1400px"
  }
  for (const header of $$("[data-header]"))
    ScrollTrigger.create({ start: 8, end: "max", onToggle: (s) => header.toggleAttribute("data-scrolled", s.isActive) })
  for (const bar of $$("[data-progress]"))
    gsap.fromTo(bar, { scaleX: 0 }, { scaleX: 1, ease: "none", scrollTrigger: { start: 0, end: "max", scrub: true } })
}

// Text the pointer can light. Hero text is strong; everything else is softer and smaller.
const LIGHTABLE = "[data-hero] [data-light], main :is(h2, h3, p, dt, dd, li > span, summary, button[data-state])"

/** Pointer light on text (fine pointers only): an RGB spot painted into the letters. */
function initTextLight() {
  if (!window.matchMedia("(hover: hover) and (pointer: fine)").matches) return
  const prepared = new WeakSet<HTMLElement>()
  let current: HTMLElement[] = []
  let frame = 0
  const prepare = (el: HTMLElement) => {
    if (prepared.has(el)) return
    prepared.add(el)
    // Keep the element's own colour as the base layer, then hand painting to the gradient.
    el.style.setProperty("--base", getComputedStyle(el).color)
    const hero = !!el.closest("[data-hero]")
    el.style.setProperty("--la", hero ? "1" : "0.9")
    el.dataset.lr = hero ? (el.tagName === "P" ? "220px" : "320px") : "260px"
    el.classList.add("lit")
  }
  const update = (x: number, y: number, target: Element | null) => {
    const hero = target?.closest("[data-hero]")
    const next = hero ? $$("[data-light]", hero) : ([target?.closest(LIGHTABLE)].filter(Boolean) as HTMLElement[])
    for (const el of current) if (!next.includes(el)) el.style.setProperty("--lr", "0px")
    for (const el of next) {
      prepare(el)
      const r = el.getBoundingClientRect()
      el.style.setProperty("--mx", `${x - r.left}px`)
      el.style.setProperty("--my", `${y - r.top}px`)
      el.style.setProperty("--lr", el.dataset.lr!)
    }
    current = next
  }
  document.addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return
    cancelAnimationFrame(frame)
    frame = requestAnimationFrame(() => update(e.clientX, e.clientY, e.target as Element))
  }, { passive: true })
  document.addEventListener("pointerleave", () => update(0, 0, null))
  for (const rgb of $$("[data-rgb]")) {
    const word = rgb.closest("[data-word]") ?? rgb
    word.addEventListener("pointerenter", () => rgb.classList.add("rgb-on"))
    word.addEventListener("pointerleave", () => rgb.classList.remove("rgb-on"))
  }
}

// Things the cursor turns towards: anything you can press, plus screenshots and images.
const AIMABLE = "a[href], button:not([disabled]), summary, [role=tab], [role=button], label[for], img, picture, [data-scrub]"
const AIM_RADIUS = 220 // px from the pointer to a target's edge
const REST = 0 // arrow points up-left by default, like the system cursor

/**
 * Custom cursor (fine pointers only): a normal arrow, drawn a little cleaner, that sits
 * exactly on the pointer (no lag) and turns its tip towards the nearest button, link or
 * image. Only the rotation is smoothed, with a critically damped spring, so it never
 * wobbles or overshoots. The tip is the hotspot and the rotation origin, so clicks land
 * exactly where the arrow points.
 */
function initCursor(calm: boolean) {
  if (!window.matchMedia("(hover: hover) and (pointer: fine)").matches) return
  const el = document.createElement("div")
  el.dataset.cursor = ""
  el.setAttribute("aria-hidden", "true")
  // Classic arrow proportions; tip at (2,2). Light fill with a dark outline reads on any background.
  el.innerHTML = `<svg viewBox="0 0 24 24" width="24" height="24">
    <defs><linearGradient id="cur-rgb" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ef4444"/><stop offset=".5" stop-color="#10a37f"/><stop offset="1" stop-color="#3b82f6"/></linearGradient></defs>
    <path class="cur-shadow" d="${ARROW}" transform="translate(.6 1.2)"/>
    <path class="cur-body" d="${ARROW}"/></svg>`
  el.style.opacity = "0"
  document.body.append(el)
  document.documentElement.classList.add("custom-cursor")

  // The arrow's own axis (tip towards the middle of the tail) in screen degrees, ~ -115.
  const AXIS = (Math.atan2(2 - 15.5, 2 - 8.5) * 180) / Math.PI
  let x = -100, y = -100, angle = REST, vel = 0, goal = REST, last = 0, raf = 0, visible = false
  let targets: HTMLElement[] = [], scanned = 0, hover: Element | null = null

  const scan = () => {
    targets = $$(AIMABLE).filter((t) => !t.closest("[data-cursor]") && t.getClientRects().length > 0)
    scanned = performance.now()
  }
  const aim = () => {
    if (performance.now() - scanned > 1000) scan()
    // Directly over a target: point at its centre.
    const over = hover?.closest?.<HTMLElement>(AIMABLE)
    let best: DOMRect | null = over ? over.getBoundingClientRect() : null
    if (!best) {
      let dist = AIM_RADIUS
      for (const t of targets) {
        const r = t.getBoundingClientRect()
        if (r.width === 0 || r.bottom < y - AIM_RADIUS || r.top > y + AIM_RADIUS) continue
        const d = Math.hypot(Math.max(r.left - x, 0, x - r.right), Math.max(r.top - y, 0, y - r.bottom))
        if (d < dist) { dist = d; best = r }
      }
    }
    if (!best) return REST
    const dx = best.left + best.width / 2 - x, dy = best.top + best.height / 2 - y
    if (Math.hypot(dx, dy) < 10) return goal // too close to have a direction: hold still
    return (Math.atan2(dy, dx) * 180) / Math.PI - AXIS
  }
  const paint = () => (el.style.transform = `translate3d(${x}px, ${y}px, 0) rotate(${angle.toFixed(2)}deg)`)
  const tick = (now: number) => {
    const dt = Math.min(0.05, (now - last) / 1000 || 0.016)
    last = now
    // Shortest way round, then a critically damped spring (no overshoot).
    const diff = ((((goal - angle) % 360) + 540) % 360) - 180
    if (calm) { angle += diff; vel = 0 }
    else {
      const k = 220
      vel += (diff * k - vel * 2 * Math.sqrt(k)) * dt
      angle += vel * dt
    }
    paint()
    raf = Math.abs(diff) > 0.05 || Math.abs(vel) > 0.05 ? requestAnimationFrame(tick) : 0
  }
  const kick = () => { if (!raf) { last = performance.now(); raf = requestAnimationFrame(tick) } }
  const show = (on: boolean) => { if (on !== visible) { visible = on; el.style.opacity = on ? "1" : "0" } }

  document.addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return
    x = e.clientX
    y = e.clientY
    hover = e.target as Element
    show(!hover.closest?.("input, textarea, select, [contenteditable]"))
    el.classList.toggle("is-link", !!hover.closest?.("a, button, summary, [role=tab], [role=button], label"))
    goal = aim()
    paint() // position follows the pointer 1:1, on the same event
    kick()
  }, { passive: true })
  // Scrolling moves targets under a still pointer: aim again.
  addEventListener("scroll", () => { if (visible) { goal = aim(); kick() } }, { passive: true })
  addEventListener("resize", scan)
  document.addEventListener("pointerdown", () => el.classList.add("is-down"))
  document.addEventListener("pointerup", () => el.classList.remove("is-down"))
  document.documentElement.addEventListener("pointerleave", () => show(false))
}

// Classic arrow outline in a 24 px box, tip at (2, 2).
const ARROW = "M2 2 L2 19.2 L6.6 15.1 L9.6 21.6 L12.6 20.3 L9.7 13.9 L15.9 13.9 Z"

/** Trackpads and precision wheels bring their own inertia: smoothing them again feels floaty. */
function wheelFilter() {
  let precise = -Infinity
  return ({ event }: { event: WheelEvent | TouchEvent }) => {
    if (!(event instanceof WheelEvent)) return true
    const fine = event.deltaMode === 0 && (event.deltaX !== 0 || !Number.isInteger(event.deltaY) || Math.abs(event.deltaY) < 40)
    if (fine) precise = event.timeStamp
    // Stay native for the rest of a trackpad gesture (its momentum tail can look like a wheel).
    return event.timeStamp - precise > 400
  }
}

export function initMotion() {
  initTextLight()
  initCursor(prefersReducedMotion())
  const calm = prefersReducedMotion()
  try {
    gsap.registerPlugin(ScrollTrigger)
    if (!calm) {
      // Mouse wheels only: each notch glides for a short, fixed time with an ease-out,
      // like the OS smooth scroll. Trackpads, touch and keyboard stay fully native.
      lenis = new Lenis({
        autoRaf: false, duration: 0.45, easing: (t) => 1 - Math.pow(1 - t, 3),
        wheelMultiplier: 1, syncTouch: false, virtualScroll: wheelFilter(),
      })
      lenis.on("scroll", ScrollTrigger.update)
      gsap.ticker.add(raf)
      gsap.ticker.lagSmoothing(0)
    }
    ctx = gsap.context(() => build(calm))
    document.addEventListener("click", onAnchorClick)
    if (location.hash) requestAnimationFrame(() => scrollToTarget(decodeURIComponent(location.hash)))
    ScrollTrigger.refresh()
  } catch (error) {
    console.warn("Motion disabled:", error)
    destroyMotion()
  } finally {
    // Content never stays hidden, even if a trigger fails.
    release()
  }
}

function raf(time: number) {
  lenis?.raf(time * 1000)
}

function onAnchorClick(event: MouseEvent) {
  if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey) return
  const link = (event.target as Element).closest?.("a[href*='#']") as HTMLAnchorElement | null
  if (!link || link.origin !== location.origin || link.pathname !== location.pathname || !link.hash) return
  const target = document.getElementById(decodeURIComponent(link.hash.slice(1)))
  if (!target) return
  event.preventDefault()
  history.pushState(null, "", link.hash)
  scrollToTarget(target)
  if (!target.hasAttribute("tabindex")) target.setAttribute("tabindex", "-1")
  target.focus({ preventScroll: true })
}

export function destroyMotion() {
  document.removeEventListener("click", onAnchorClick)
  ctx?.revert()
  ScrollTrigger.getAll().forEach((t) => t.kill())
  gsap.ticker.remove(raf)
  lenis?.destroy()
  lenis = null
  ctx = null
}
