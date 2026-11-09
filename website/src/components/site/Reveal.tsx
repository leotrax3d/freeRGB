import type { HTMLAttributes } from "react"

type Kind = "reveal" | "batch" | "scale"
const attr: Record<Kind, string> = { reveal: "data-reveal", batch: "data-batch", scale: "data-reveal-scale" }

/** Marks a block for scroll reveal (see src/lib/motion.ts). A plain div; no effect on layout. */
export function Reveal({ kind = "reveal", ...props }: HTMLAttributes<HTMLDivElement> & { kind?: Kind }) {
  return <div {...props} {...{ [attr[kind]]: "" }} />
}
