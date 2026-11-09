import { useEffect, type RefObject } from "react"
import { parallax } from "@/lib/motion"

/** Scroll-linked parallax (+-distance px) from 768 px up; no-op with reduced motion. */
export function useParallax(ref: RefObject<HTMLElement>, distance = 30) {
  useEffect(() => (ref.current ? parallax(ref.current, distance) : undefined), [ref, distance])
}
