import { ExternalLink } from "lucide-react"
import { DeviceSearch } from "@/components/devices/DeviceSearch"
import { FanChips } from "@/components/devices/FanChips"
import { PageHeader, Section } from "@/components/site/Section"
import { Button } from "@/components/ui/button"
import { devicesPage, repo } from "@/content/site"

// Page-local copy (rules: header of content/site.ts).
const reportLabel = "Report what works"

export function DevicesPage() {
  return (
    <>
      <PageHeader title={devicesPage.title} lead={devicesPage.lead} />

      <div className="container">
        <DeviceSearch />
      </div>

      <Section id="fans" labelledBy="fans-title" className="scroll-mt-20">
        <FanChips />
      </Section>

      <div className="container">
        <div className="flex flex-col gap-5 rounded-card border bg-card p-6 md:flex-row md:items-center md:justify-between md:p-8">
          <p className="max-w-2xl text-body text-muted-foreground">{devicesPage.note}</p>
          <Button variant="outline" size="lg" className="self-start md:self-auto" asChild>
            <a href={`${repo}/issues`}>
              {reportLabel}
              <ExternalLink aria-hidden />
            </a>
          </Button>
        </div>
      </div>
    </>
  )
}
