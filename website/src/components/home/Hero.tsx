import { ArrowUpRight } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Screenshot } from "@/components/site/Screenshot"
import { DownloadButton } from "@/components/home/DownloadButton"
import { hero } from "@/content/site"
import { href } from "@/lib/paths"

// Badge merges classes with tailwind-merge, which drops the custom text-caption size next to
// a colour class. The arbitrary values below restate the caption token so it survives.
const caption = "text-[12px] leading-4 font-medium tracking-[0.01em]"

export function Hero() {
  return (
    <div className="relative overflow-hidden">
      {/* Faint light from the top edge. Neutral colour, no hue. */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-x-0 top-0 h-[560px]"
        style={{ background: "radial-gradient(ellipse 60% 100% at 50% 0%, hsl(var(--foreground) / 0.06), transparent 70%)" }}
      />
      <div className="container relative pb-16 pt-16 md:pb-24 md:pt-24">
        <div className="mx-auto flex max-w-3xl flex-col items-center text-center">
          <Badge variant="muted" className={`gap-2 rounded-full px-3 py-1 ${caption}`}>
            <span aria-hidden className="size-1.5 rounded-full bg-warning" />
            {hero.badge}
          </Badge>
          <h1 className="mt-6 text-balance text-display">{hero.title}</h1>
          <p className="mt-6 max-w-2xl text-pretty text-lead text-muted-foreground">{hero.lead}</p>
          <div className="mt-9 flex w-full flex-col items-stretch justify-center gap-3 sm:w-auto sm:flex-row sm:items-center">
            <DownloadButton />
            <Button size="lg" variant="outline" asChild>
              <a href={href("demo/")} target="_blank" rel="noopener">
                {hero.secondary}
                <ArrowUpRight aria-hidden />
                <span className="sr-only">(opens in a new tab)</span>
              </a>
            </Button>
          </div>
          <p className="mt-5 text-small text-subtle-foreground">{hero.note}</p>
        </div>

        <div className="relative mx-auto mt-16 max-w-[1100px] md:mt-20">
          <div aria-hidden className="rgb-glow animate-glow-hue z-0" style={{ left: "-10%", right: "-10%", top: "10%", bottom: "20%" }} />
          <div className="relative z-10 rounded-panel border bg-card p-2 shadow-frame">
            <Screenshot name="lighting" alt={hero.screenshotAlt} priority sizes="(min-width: 1200px) 1100px, 100vw" className="rounded-[12px]" />
          </div>
        </div>
      </div>
    </div>
  )
}
