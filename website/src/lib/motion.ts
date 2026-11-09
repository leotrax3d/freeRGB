// Scroll motion for the website: Lenis smooth scroll + GSAP ScrollTrigger.
// Driven by data attributes in the prerendered markup, so nothing is hidden in the
// static HTML. public/theme-init.js adds `motion-pending` to <html> (JS on, motion
// allowed); CSS hides reveal targets only under that class until initMotion() runs.
//
//   data-hero          children reveal in sequence on load ([data-word] spans are split words)
//   data-hero-shot     rises in with scale 0.96 -> 1
//   data-glow-parallax moves slower than the page
//   data-reveal        children fade/translate up on enter, staggered
//   data-batch         children revealed with ScrollTrigger.batch
//   data-feature       two children slide in from opposite sides (data-flipped swaps them)
//   data-reveal-scale  scale/opacity reveal
//   data-scrub         scroll-linked parallax of +-30 px (768 px and up)
//   data-header        gets data-scrolled after 8 px
//   data-progress      scaleX scroll progress
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

const up = { opacity: 0, y: 24 }
const done = { clearProps: "willChange" }
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

function reveal(targets: Element[], trigger: Element) {
  if (!targets.length) return
  gsap.fromTo(targets, up, {
    opacity: 1, y: 0, duration: 0.8, ease: "power3.out", stagger: 0.08, willChange: "transform,opacity",
    scrollTrigger: { trigger, start: "top 85%", once: true }, ...done,
  })
}

function build() {
  for (const hero of $$("[data-hero]")) {
    const items = Array.from(hero.children).flatMap((c) => (c.querySelector("[data-word]") ? $$("[data-word]", c) : [c]))
    gsap.fromTo(items, up, { opacity: 1, y: 0, duration: 0.9, ease: "expo.out", stagger: 0.05, delay: 0.05, ...done })
  }
  for (const shot of $$("[data-hero-shot]"))
    gsap.fromTo(shot, { opacity: 0, y: 40, scale: 0.96 }, { opacity: 1, y: 0, scale: 1, duration: 0.9, delay: 0.35, ease: "expo.out", ...done })
  for (const el of $$("[data-glow-parallax]"))
    gsap.to(el, { yPercent: 25, ease: "none", scrollTrigger: { trigger: el.parentElement, start: "top top", end: "bottom top", scrub: true } })
  for (const el of $$("[data-reveal]")) reveal(Array.from(el.children), el)
  for (const el of $$("[data-reveal-scale]"))
    gsap.fromTo(el, { opacity: 0, scale: 0.96 }, { opacity: 1, scale: 1, duration: 0.8, ease: "power3.out", scrollTrigger: { trigger: el, start: "top 85%", once: true }, ...done })
  for (const el of $$("[data-feature]")) {
    const [text, shot] = Array.from(el.children)
    const side = el.hasAttribute("data-flipped") ? -1 : 1
    gsap.matchMedia().add({ wide: "(min-width: 1024px)" }, (c) => {
      const x = c.conditions?.wide ? 40 : 0
      for (const [node, sign] of [[text, -side], [shot, side]] as const)
        if (node)
          gsap.fromTo(node, { opacity: 0, x: x * sign, y: x ? 0 : 24 }, { opacity: 1, x: 0, y: 0, duration: 0.9, ease: "power3.out", scrollTrigger: { trigger: el, start: "top 80%", once: true }, ...done })
    })
  }
  for (const list of $$("[data-batch]")) {
    const items = Array.from(list.children)
    gsap.set(items, up)
    ScrollTrigger.batch(items, {
      start: "top 90%", once: true,
      onEnter: (batch) => { gsap.to(batch, { opacity: 1, y: 0, duration: 0.7, ease: "power3.out", stagger: 0.08, overwrite: true }) },
    })
  }
  for (const el of $$("[data-scrub]")) parallax(el)
  for (const header of $$("[data-header]"))
    ScrollTrigger.create({ start: 8, end: "max", onToggle: (s) => header.toggleAttribute("data-scrolled", s.isActive) })
  for (const bar of $$("[data-progress]"))
    gsap.fromTo(bar, { scaleX: 0 }, { scaleX: 1, ease: "none", scrollTrigger: { start: 0, end: "max", scrub: 0.2 } })
}

export function initMotion() {
  if (prefersReducedMotion()) return release()
  try {
    gsap.registerPlugin(ScrollTrigger)
    lenis = new Lenis({ autoRaf: false })
    lenis.on("scroll", ScrollTrigger.update)
    gsap.ticker.add(raf)
    gsap.ticker.lagSmoothing(0)
    ctx = gsap.context(build)
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
