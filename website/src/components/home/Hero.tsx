import { Fragment } from "react"
import { WordDrum } from "@/components/home/WordDrum"
import { ArrowUpRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Screenshot } from "@/components/site/Screenshot"
import { DownloadButton } from "@/components/home/DownloadButton"
import { hero } from "@/content/site"
import { href } from "@/lib/paths"

export function Hero() {
  return (
    <div className="relative overflow-hidden">
      <div className="container relative pb-16 pt-16 md:pb-24 md:pt-24">
        <div data-hero="" className="mx-auto flex max-w-5xl flex-col items-center text-center">
          <h1 className="text-[clamp(38px,9.5vw,120px)] font-semibold leading-[0.98] tracking-[-0.05em]">
            <span className="sr-only">{hero.title}</span>
            <span aria-hidden>
              {"One app for your".split(" ").map((word, i) => (
                <Fragment key={i}>
                  {i > 0 && " "}
                  <span data-word="" data-light="" className="inline-block">
                    {word}
                  </span>
                </Fragment>
              ))}
              <br />
              <span data-word="" className="inline-block">
                <WordDrum />
              </span>
            </span>
          </h1>
          <p data-light="" className="mt-6 max-w-2xl text-pretty text-lead text-muted-foreground">
            {hero.lead}
          </p>
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
          <div data-hero-shot="" className="relative z-10 rounded-panel border bg-card p-2 shadow-frame">
            <Screenshot name="lighting" alt={hero.screenshotAlt} priority sizes="(min-width: 1200px) 1100px, 100vw" className="rounded-[12px]" />
          </div>
        </div>
      </div>
    </div>
  )
}
