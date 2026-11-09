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

const scrub = (trigger: Element, start: string, end: string) => ({ trigger, start, end, scrub: 0.4 })

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
        scrollTrigger: { trigger: el, start: "top 90%", end: wide ? "center 55%" : "top 30%", scrub: 0.6 },
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
    const st = { trigger: list, start: "top bottom", end: "top 60%", scrub: 0.5 }
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
    gsap.fromTo(bar, { scaleX: 0 }, { scaleX: 1, ease: "none", scrollTrigger: { start: 0, end: "max", scrub: 0.2 } })
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

/** Custom cursor: the freeRGB ring (red, green, blue arcs) Fine pointers only. */
function initCursor(calm: boolean) {
  if (!window.matchMedia("(hover: hover) and (pointer: fine)").matches) return
  const ring = document.createElement("div")
  ring.dataset.cursor = ""
  ring.setAttribute("aria-hidden", "true")
  // Three arcs with gaps, like the logo: r=10, circumference ~62.8, each arc 17 with a 3.9 gap.
  ring.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke-width="2.4" stroke-linecap="round">
    <circle cx="12" cy="12" r="10" stroke="#ef4444" stroke-dasharray="17 45.8" stroke-dashoffset="0"/>
    <circle cx="12" cy="12" r="10" stroke="#10a37f" stroke-dasharray="17 45.8" stroke-dashoffset="-20.9"/>
    <circle cx="12" cy="12" r="10" stroke="#3b82f6" stroke-dasharray="17 45.8" stroke-dashoffset="-41.8"/></svg>`
  ring.style.opacity = "0"
  document.body.append(ring)
  document.documentElement.classList.add("ring-cursor")
  const lag = calm ? 0 : 0.18
  const rx = gsap.quickTo(ring, "x", { duration: lag, ease: "power3.out" }), ry = gsap.quickTo(ring, "y", { duration: lag, ease: "power3.out" })
  document.addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return
    rx(e.clientX)
    ry(e.clientY)
    const t = e.target as Element
    ring.classList.toggle("is-link", !!t.closest?.("a, button, summary, [role=tab], label"))
    const text = !!t.closest?.("input, textarea, select, [contenteditable]")
    ring.style.opacity = text ? "0" : "1"
  }, { passive: true })
  document.addEventListener("pointerdown", () => ring.classList.add("is-down"))
  document.addEventListener("pointerup", () => ring.classList.remove("is-down"))
  document.documentElement.addEventListener("pointerleave", () => (ring.style.opacity = "0"))
}

export function initMotion() {
  initTextLight()
  initCursor(prefersReducedMotion())
  const calm = prefersReducedMotion()
  try {
    gsap.registerPlugin(ScrollTrigger)
    if (!calm) {
      // Light smoothing for the wheel only, with quick catch-up so scrolling never drags.
      lenis = new Lenis({ autoRaf: false, lerp: 0.18, wheelMultiplier: 1.05, syncTouch: false })
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
