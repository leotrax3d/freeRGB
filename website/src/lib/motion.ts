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

function reveal(targets: Element[], trigger: Element, calm: boolean) {
  if (!targets.length) return
  gsap.fromTo(targets, calm ? { opacity: 0 } : { opacity: 0, y: 14 }, {
    opacity: 1, y: 0, duration: calm ? 0.4 : 0.55, ease: "power2.out", stagger: 0.05,
    scrollTrigger: { trigger, start: "top 92%", once: true }, ...done,
  })
}

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
    else gsap.fromTo(el, { scale: 0.86, opacity: 0.3 }, { scale: 1, opacity: 1, ease: "none", scrollTrigger: scrub(el, "top bottom", "top 55%") })
  }
  const mm = gsap.matchMedia()
  for (const el of $$("[data-feature]")) {
    const [text, media] = Array.from(el.children) as HTMLElement[]
    if (text) reveal(Array.from(text.children), el, calm)
    const frame = media?.querySelector<HTMLElement>("[data-scrub]")
    if (!media || !frame) continue
    if (calm) { reveal([media], el, true); continue }
    gsap.fromTo(frame, { clipPath: "inset(10% 14% 10% 14% round 20px)" }, { clipPath: "inset(0% 0% 0% 0% round 14px)", ease: "none", scrollTrigger: scrub(frame, "top 95%", "center 55%") })
    const img = frame.querySelector("img")
    if (img) gsap.fromTo(img, { scale: 1.18 }, { scale: 1, ease: "none", scrollTrigger: scrub(frame, "top 95%", "center 55%") })
    mm.add("(min-width: 768px)", () => {
      gsap.fromTo(media, { y: 30 }, { y: -30, ease: "none", scrollTrigger: { trigger: el, start: "top bottom", end: "bottom top", scrub: true } })
    })
  }
  for (const list of $$("[data-batch]")) {
    const items = Array.from(list.children)
    if (calm) reveal(items, list, true)
    else gsap.fromTo(items, { y: 60, opacity: 0 }, { y: 0, opacity: 1, ease: "none", stagger: 0.12, scrollTrigger: { trigger: list, start: "top 98%", end: "top 60%", scrub: 0.5 } })
  }
  for (const header of $$("[data-header]"))
    ScrollTrigger.create({ start: 8, end: "max", onToggle: (s) => header.toggleAttribute("data-scrolled", s.isActive) })
  for (const bar of $$("[data-progress]"))
    gsap.fromTo(bar, { scaleX: 0 }, { scaleX: 1, ease: "none", scrollTrigger: { start: 0, end: "max", scrub: 0.2 } })
}

export function initMotion() {
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
