import fansData from "virtual:fans"
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion"
import { Badge } from "@/components/ui/badge"
import { devicesPage } from "@/content/site"
import { cn } from "@/lib/utils"

type Chip = (typeof fansData.chips)[number]

// Page-local copy (rules: header of content/site.ts).
const copy = {
  controllable: "With fan speed control",
  readOnly: "Read-only or not supported",
  chipsTotal: "Chips listed",
  controlTableCaption: "Fan controller chips with fan speed control",
  readOnlyTableCaption: "Fan controller chips without fan speed control",
  columns: { chip: "Chip", vendor: "Vendor", boards: "Typical boards", headers: "Fan headers", status: "Status" },
  notListed: "Not listed",
  headers: (fans: number, controls: number) => `${fans} ${fans === 1 ? "fan" : "fans"} · ${controls} ${controls === 1 ? "control" : "controls"}`,
  source: (version: string) => `Chip list from LibreHardwareMonitor ${version}. Board names are examples, not a complete list.`,
}

const compare = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0)
const isControllable = (c: Chip) => c.controls > 0 && c.status !== "unsupported"
const byVendorName = (a: Chip, b: Chip) => compare(a.vendor, b.vendor) || compare(a.name, b.name)

const controllable = fansData.chips.filter(isControllable).sort(byVendorName)
const readOnly = fansData.chips.filter((c) => !isControllable(c)).sort(byVendorName)

function FanStatus({ chip }: { chip: Chip }) {
  const unsupported = chip.status === "unsupported"
  return (
    <Badge variant={unsupported ? "muted" : "brand"} className={cn("whitespace-nowrap text-left", unsupported && "text-subtle-foreground")}>
      {devicesPage.fanStatus[chip.status] ?? devicesPage.fanStatus.unsupported}
    </Badge>
  )
}

const boardsText = (chip: Chip) => (chip.boards.length ? chip.boards.join(", ") : copy.notListed)

function ChipTable({ chips, caption }: { chips: Chip[]; caption: string }) {
  const th = "px-4 py-3 text-left text-caption font-medium uppercase tracking-wider text-subtle-foreground"
  return (
    <>
      <div className="hidden overflow-x-auto rounded-card border bg-card lg:block">
        <table className="w-full table-fixed border-collapse text-small">
          <caption className="sr-only">{caption}</caption>
          <colgroup>
            <col className="w-[22%]" />
            <col className="w-[13%]" />
            <col className="w-[29%]" />
            <col className="w-[15%]" />
            <col className="w-[21%]" />
          </colgroup>
          <thead className="border-b">
            <tr>
              <th scope="col" className={th}>{copy.columns.chip}</th>
              <th scope="col" className={th}>{copy.columns.vendor}</th>
              <th scope="col" className={th}>{copy.columns.boards}</th>
              <th scope="col" className={th}>{copy.columns.headers}</th>
              <th scope="col" className={th}>{copy.columns.status}</th>
            </tr>
          </thead>
          <tbody>
            {chips.map((c) => (
              <tr key={c.chip} className="border-b align-top last:border-b-0">
                <th scope="row" className="break-words px-4 py-3 text-left font-medium text-foreground">
                  {c.name}
                </th>
                <td className="break-words px-4 py-3 text-muted-foreground">{c.vendor}</td>
                <td className={cn("break-words px-4 py-3", c.boards.length ? "text-muted-foreground" : "text-subtle-foreground")}>{boardsText(c)}</td>
                <td className="px-4 py-3 tabular-nums text-muted-foreground">{copy.headers(c.fans, c.controls)}</td>
                <td className="px-4 py-3">
                  <FanStatus chip={c} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <ul className="space-y-3 lg:hidden" aria-label={caption}>
        {chips.map((c) => (
          <li key={c.chip} className="rounded-card border bg-card p-4">
            <div className="flex flex-col gap-2">
              <p className="break-words text-body font-medium text-foreground">{c.name}</p>
              <FanStatus chip={c} />
            </div>
            <dl className="mt-3 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1.5 text-small">
              <dt className="text-subtle-foreground">{copy.columns.vendor}</dt>
              <dd className="min-w-0 text-muted-foreground">{c.vendor}</dd>
              <dt className="text-subtle-foreground">{copy.columns.boards}</dt>
              <dd className="min-w-0 break-words text-muted-foreground">{boardsText(c)}</dd>
              <dt className="text-subtle-foreground">{copy.columns.headers}</dt>
              <dd className="min-w-0 text-muted-foreground">{copy.headers(c.fans, c.controls)}</dd>
            </dl>
          </li>
        ))}
      </ul>
    </>
  )
}

export function FanChips() {
  const stats = [
    { label: copy.controllable, value: controllable.length },
    { label: copy.readOnly, value: readOnly.length },
    { label: copy.chipsTotal, value: fansData.chips.length },
  ]
  return (
    <div>
      <h2 id="fans-title" className="text-h2">
        {devicesPage.fansTitle}
      </h2>
      <p className="mt-4 max-w-2xl text-lead text-muted-foreground">{devicesPage.fansLead}</p>

      <dl className="mt-8 grid grid-cols-1 gap-3 sm:grid-cols-3">
        {stats.map((s) => (
          <div key={s.label} className="rounded-card border bg-card p-5">
            <dt className="text-small text-muted-foreground">{s.label}</dt>
            <dd className="mt-1 text-h2 tabular-nums text-foreground">{s.value}</dd>
          </div>
        ))}
      </dl>

      <h3 className="mt-12 text-h3">
        {copy.controllable} <span className="font-normal text-subtle-foreground">({controllable.length})</span>
      </h3>
      <div className="mt-4">
        <ChipTable chips={controllable} caption={copy.controlTableCaption} />
      </div>

      {readOnly.length > 0 && (
        <Accordion type="single" collapsible className="mt-10 border-t">
          <AccordionItem value="read-only">
            <AccordionTrigger className="min-h-11 text-body">
              {copy.readOnly} ({readOnly.length})
            </AccordionTrigger>
            <AccordionContent>
              <ChipTable chips={readOnly} caption={copy.readOnlyTableCaption} />
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}

      <p className="mt-6 text-small text-subtle-foreground">{copy.source(fansData.lhmVersion)}</p>
    </div>
  )
}
