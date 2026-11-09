import { Footprint } from "@/components/home/Footprint"
import { Reveal } from "@/components/site/Reveal"
import { ArrowRight, Boxes, Cable, Laptop, Puzzle, Terminal, type LucideIcon } from "lucide-react"
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion"
import { Button } from "@/components/ui/button"
import { Eyebrow, Section } from "@/components/site/Section"
import { DeviceTeaser } from "@/components/home/DeviceTeaser"
import { DownloadButton } from "@/components/home/DownloadButton"
import { Features } from "@/components/home/Features"
import { Hero } from "@/components/home/Hero"
import { TrustStrip } from "@/components/home/TrustStrip"
import { faq, platform, platforms } from "@/content/site"
import { href } from "@/lib/paths"

// Page-local copy (not in content/site.ts). Factual, see the rules in that file's header.
const copy = {
  platformEyebrow: "Platform",
  platformsEyebrow: "Platforms",
  platformsTitle: "Where freeRGB runs",
  platformLinks: { windows: "Windows download", linux: "Linux install steps" } as Record<string, string>,
  faqTitle: "Questions",
  faqMore: "More answers are in the user guide.",
  faqMoreLink: "Open the user guide",
  ctaTitle: "Try it on your PC",
  ctaText: "Beta builds for Windows 10 and 11, source builds for Linux. Read the beta notice on the download page before you install.",
  ctaGuide: "Read the user guide",
}

const platformIcons: LucideIcon[] = [Cable, Boxes, Puzzle]
const osIcons: Record<string, LucideIcon> = { windows: Laptop, linux: Terminal }

function PlatformBlock() {
  return (
    <Section labelledBy="platform-title">
      <div data-reveal="" className="max-w-2xl">
        <Eyebrow>{copy.platformEyebrow}</Eyebrow>
        <h2 id="platform-title" className="mt-3 text-h2">
          {platform.title}
        </h2>
        <p className="mt-4 text-body text-muted-foreground">{platform.text}</p>
      </div>
      <ul data-batch="" className="mt-10 grid gap-4 md:grid-cols-3">
        {platform.points.map((point, i) => {
          const Icon = platformIcons[i] ?? Cable
          return (
            <li key={point.title} className="rounded-card border bg-card p-6">
              <Icon aria-hidden className="size-5 text-muted-foreground" strokeWidth={1.75} />
              <h3 className="mt-4 text-h3">{point.title}</h3>
              <p className="mt-2 text-small text-muted-foreground">{point.text}</p>
            </li>
          )
        })}
      </ul>
    </Section>
  )
}

function Platforms() {
  return (
    <Section labelledBy="platforms-title">
      <Reveal>
        <Eyebrow>{copy.platformsEyebrow}</Eyebrow>
        <h2 id="platforms-title" className="mt-3 text-h2">
          {copy.platformsTitle}
        </h2>
      </Reveal>
      <ul data-batch="" className="mt-10 grid gap-4 md:grid-cols-2">
        {platforms.map((os) => {
          const Icon = osIcons[os.id] ?? Laptop
          return (
            <li key={os.id} className="flex flex-col rounded-card border bg-card p-6 md:p-8">
              <Icon aria-hidden className="size-5 text-muted-foreground" strokeWidth={1.75} />
              <h3 className="mt-4 text-h3">{os.title}</h3>
              <p className="mt-2 flex-1 text-body text-muted-foreground">{os.text}</p>
              <a
                href={href(`download/#${os.id}`)}
                className="mt-6 inline-flex min-h-11 items-center gap-1.5 self-start rounded-md text-small font-medium text-brand-foreground underline-offset-4 hover:underline"
              >
                {copy.platformLinks[os.id]}
                <ArrowRight aria-hidden className="size-4" />
              </a>
            </li>
          )
        })}
      </ul>
    </Section>
  )
}

function Faq() {
  return (
    <Section labelledBy="faq-title">
      <div data-reveal="" className="grid gap-10 lg:grid-cols-[1fr_2fr] lg:gap-16">
        <div>
          <h2 id="faq-title" className="text-h2">
            {copy.faqTitle}
          </h2>
          <p className="mt-4 text-body text-muted-foreground">{copy.faqMore}</p>
          <a
            href={href("docs/")}
            className="mt-2 inline-flex min-h-11 items-center gap-1.5 rounded-md text-small font-medium text-brand-foreground underline-offset-4 hover:underline"
          >
            {copy.faqMoreLink}
            <ArrowRight aria-hidden className="size-4" />
          </a>
        </div>
        <Accordion type="multiple" className="border-t">
          {faq.map((item, i) => (
            <AccordionItem key={item.q} value={`faq-${i}`}>
              <AccordionTrigger className="min-h-11 gap-4 py-5 hover:no-underline [&>svg]:size-5">
                <span className="text-body font-medium">{item.q}</span>
              </AccordionTrigger>
              <AccordionContent className="pb-6">
                <p className="max-w-2xl text-body text-muted-foreground">{item.a}</p>
              </AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>
      </div>
    </Section>
  )
}

function FinalCta() {
  return (
    <Section labelledBy="cta-title">
      <div data-reveal-scale="" className="flex flex-col items-center rounded-panel border bg-card px-6 py-12 text-center md:px-12 md:py-16">
        <h2 id="cta-title" className="text-h2">
          {copy.ctaTitle}
        </h2>
        <p className="mt-4 max-w-xl text-body text-muted-foreground">{copy.ctaText}</p>
        <div className="mt-8 flex w-full flex-col items-stretch gap-3 sm:w-auto sm:flex-row sm:items-center">
          <DownloadButton />
          <Button size="lg" variant="outline" asChild>
            <a href={href("docs/")}>{copy.ctaGuide}</a>
          </Button>
        </div>
      </div>
    </Section>
  )
}

export function HomePage() {
  return (
    <>
      <Hero />
      <Footprint />
      <TrustStrip />
      <DeviceTeaser />
      <Features />
      <PlatformBlock />
      <Platforms />
      <Faq />
      <FinalCta />
    </>
  )
}
