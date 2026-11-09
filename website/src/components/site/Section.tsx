import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

/** Page title block used at the top of every sub-page. */
export function PageHeader({ title, lead, children }: { title: string; lead: string; children?: ReactNode }) {
  return (
    <div data-reveal="" className="container pb-12 pt-16 md:pb-16 md:pt-24">
      <h1 className="max-w-3xl text-h1">{title}</h1>
      <p className="mt-5 max-w-2xl text-lead text-muted-foreground">{lead}</p>
      {children}
    </div>
  )
}

/** Section with the standard vertical rhythm (96 px desktop, 64 px mobile). */
export function Section({ id, className, children, labelledBy }: { id?: string; className?: string; children: ReactNode; labelledBy?: string }) {
  return (
    <section id={id} aria-labelledby={labelledBy} className={cn("container py-16 md:py-24", className)}>
      {children}
    </section>
  )
}

export function Eyebrow({ children }: { children: ReactNode }) {
  return <p className="text-caption uppercase tracking-wider text-brand-foreground">{children}</p>
}
