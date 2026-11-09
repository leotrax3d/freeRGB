import { Check } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Eyebrow } from "@/components/site/Section"
import { Screenshot } from "@/components/site/Screenshot"
import { ColorDemo } from "@/components/home/ColorDemo"
import { features } from "@/content/site"
import { cn } from "@/lib/utils"

// See Hero.tsx: restates the caption size so tailwind-merge in Badge keeps it.
const caption = "text-[12px] leading-4 font-medium tracking-[0.01em]"

export function Features() {
  return (
    <>
      {features.map((feature, index) => {
        const titleId = `feature-${feature.id}-title`
        const flipped = index % 2 === 1
        return (
          <section key={feature.id} id={feature.id} aria-labelledby={titleId} className="container py-16 md:py-24">
            <div data-feature="" data-flipped={flipped ? "" : undefined} className="grid gap-10 lg:grid-cols-2 lg:items-center lg:gap-16">
              <div className={cn("max-w-xl", flipped && "lg:order-last")}>
                <Eyebrow>{feature.eyebrow}</Eyebrow>
                <h2 id={titleId} className="mt-3 text-h2">
                  {feature.title}
                </h2>
                {feature.maturity && (
                  <Badge variant="warning" className={cn("mt-4 h-auto py-1", caption)}>
                    {feature.maturity}
                  </Badge>
                )}
                <p className="mt-4 text-body text-muted-foreground">{feature.text}</p>
                <ul className="mt-6 space-y-3">
                  {feature.points.map((point) => (
                    <li key={point} className="flex gap-3 text-body">
                      <Check aria-hidden className="mt-[5px] size-4 shrink-0 text-brand-foreground" strokeWidth={2.25} />
                      <span>{point}</span>
                    </li>
                  ))}
                </ul>
                {feature.id === "lighting" && <ColorDemo className="mt-8" />}
              </div>
              <div>
              <div data-scrub="" className="rounded-card border bg-card p-1.5">
                <Screenshot name={feature.screenshot} alt={feature.alt} sizes="(min-width: 1024px) 560px, 100vw" className="rounded-[9px]" />
              </div>
              </div>
            </div>
          </section>
        )
      })}
    </>
  )
}
