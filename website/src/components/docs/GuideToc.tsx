import { useEffect, useState } from "react"
import { ChevronDown } from "lucide-react"
import { cn } from "@/lib/utils"

export type TocEntry = { id: string; text: string; level: number }

const TOC_LABEL = "On this page"

/**
 * Tracks which guide heading is currently being read. Runs only after mount, so the
 * server render and the first client render are identical (no active entry).
 */
export function useActiveHeading(ids: string[]) {
  const [active, setActive] = useState<string | null>(null)

  useEffect(() => {
    if (typeof IntersectionObserver === "undefined") return
    const headings = ids.map((id) => document.getElementById(id)).filter((el): el is HTMLElement => el !== null)
    if (headings.length === 0) return

    const visible = new Set<string>()
    const pick = () => {
      // First visible heading in document order; otherwise the last heading above the viewport.
      const first = headings.find((h) => visible.has(h.id))
      if (first) return setActive(first.id)
      const above = headings.filter((h) => h.getBoundingClientRect().top < 0)
      setActive(above.length > 0 ? above[above.length - 1].id : null)
    }

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) visible.add(entry.target.id)
          else visible.delete(entry.target.id)
        }
        pick()
      },
      // Below the 64 px sticky header, upper part of the viewport only.
      { rootMargin: "-80px 0px -55% 0px", threshold: 0 },
    )
    headings.forEach((h) => observer.observe(h))
    return () => observer.disconnect()
  }, [ids])

  return active
}

function TocList({ toc, active }: { toc: TocEntry[]; active: string | null }) {
  return (
    <ul className="space-y-0.5 border-l text-small">
      {toc.map((entry) => {
        const current = entry.id === active
        return (
          <li key={entry.id}>
            <a
              href={`#${entry.id}`}
              aria-current={current ? "location" : undefined}
              className={cn(
                "-ml-px block border-l py-2 pr-2 text-muted-foreground transition-colors duration-fast ease-out hover:text-foreground lg:py-1.5",
                entry.level >= 3 ? "pl-7" : "pl-4",
                current ? "border-brand font-medium text-foreground" : "border-transparent",
              )}
            >
              {entry.text}
            </a>
          </li>
        )
      })}
    </ul>
  )
}

/** Sticky table of contents for lg screens. */
export function GuideTocDesktop({ toc, active }: { toc: TocEntry[]; active: string | null }) {
  return (
    <nav aria-label={TOC_LABEL} className="sticky top-24 max-h-[calc(100vh-8rem)] overflow-y-auto pb-8">
      <p className="mb-3 text-caption uppercase tracking-wider text-subtle-foreground">{TOC_LABEL}</p>
      <TocList toc={toc} active={active} />
    </nav>
  )
}

/** Collapsible table of contents shown above the article below lg. */
export function GuideTocMobile({ toc, active }: { toc: TocEntry[]; active: string | null }) {
  return (
    <details className="group rounded-card border bg-card">
      <summary className="flex min-h-11 cursor-pointer list-none items-center justify-between gap-4 rounded-card px-4 py-3 text-small font-medium text-foreground [&::-webkit-details-marker]:hidden">
        {TOC_LABEL}
        <ChevronDown aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground transition-transform duration-fast ease-out group-open:rotate-180" />
      </summary>
      <nav aria-label={TOC_LABEL} className="px-4 pb-4">
        <TocList toc={toc} active={active} />
      </nav>
    </details>
  )
}
