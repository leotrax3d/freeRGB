import { useDeferredValue, useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react"
import { Search, X } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { devicesPage } from "@/content/site"
import { cn } from "@/lib/utils"
import {
  formatCount,
  matchesStatus,
  matchQuery,
  queryWords,
  transportLabel,
  transports,
  type Device,
  type DeviceStatus,
  type StatusFilter,
} from "./deviceData"

const PAGE_SIZE = 60

// Page-local copy (rules: header of content/site.ts).
const copy = {
  heading: "Device list",
  statusGroup: "Support status",
  statusFilters: [
    { id: "supported", label: "Supported" },
    { id: "unsupported", label: "Not supported yet" },
    { id: "all", label: "All" },
  ] as { id: StatusFilter; label: string }[],
  transportGroup: "Connection",
  transportAll: "Any",
  clear: "Clear search",
  count: (n: number) => `${formatCount(n)} ${n === 1 ? "device" : "devices"}`,
  showing: (shown: number, total: number) => `Showing ${formatCount(shown)} of ${formatCount(total)}.`,
  showMore: (n: number) => `Show ${n} more`,
  otherMatches: (n: number) => `${formatCount(n)} more ${n === 1 ? "match is" : "matches are"} in another status.`,
  showAll: "Show all statuses",
  tableCaption: "Devices matching the search, supported devices first",
  columns: { device: "Device", family: "Family / module", connection: "Connection", ids: "USB / PCI IDs", status: "Status" },
  noIds: "Not listed",
  noModule: "No module",
  legendTitle: "Status labels",
}

const STATUS_KEY: Record<DeviceStatus, "supported" | "blocked" | "missing"> = { s: "supported", b: "blocked", n: "missing" }

export function StatusBadge({ status, className }: { status: DeviceStatus; className?: string }) {
  const key = STATUS_KEY[status]
  return (
    <Badge
      variant={status === "s" ? "brand" : status === "b" ? "outline" : "muted"}
      className={cn("whitespace-nowrap text-left", status === "n" && "text-subtle-foreground", className)}
    >
      {devicesPage.statusLabels[key]}
    </Badge>
  )
}

function Chip({ pressed, onClick, children }: { pressed: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className={cn(
        "inline-flex h-11 items-center rounded-md border px-3.5 text-small text-muted-foreground transition-colors duration-fast ease-out md:h-9",
        "hover:border-border-strong hover:text-foreground",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        pressed && "border-border-strong bg-secondary font-medium text-foreground",
      )}
    >
      {children}
    </button>
  )
}

function Ids({ ids }: { ids: string }) {
  if (!ids) return <span className="text-subtle-foreground">{copy.noIds}</span>
  return (
    <span className="block break-all font-mono text-[13px] leading-5" title={ids}>
      {ids}
    </span>
  )
}

function FamilyModule({ device }: { device: Device }) {
  return (
    <>
      <span className="block break-words">{device.family}</span>
      <span className="block break-words text-caption font-normal text-subtle-foreground">{device.module || copy.noModule}</span>
    </>
  )
}

function ResultsTable({ rows }: { rows: Device[] }) {
  const th = "px-4 py-3 text-left text-caption font-medium uppercase tracking-wider text-subtle-foreground"
  return (
    <div className="hidden overflow-x-auto rounded-card border bg-card lg:block">
      <table className="w-full table-fixed border-collapse text-small">
        <caption className="sr-only">{copy.tableCaption}</caption>
        <colgroup>
          <col className="w-[28%]" />
          <col className="w-[19%]" />
          <col className="w-[15%]" />
          <col className="w-[17%]" />
          <col className="w-[21%]" />
        </colgroup>
        <thead className="border-b">
          <tr>
            <th scope="col" className={th}>{copy.columns.device}</th>
            <th scope="col" className={th}>{copy.columns.family}</th>
            <th scope="col" className={th}>{copy.columns.connection}</th>
            <th scope="col" className={th}>{copy.columns.ids}</th>
            <th scope="col" className={th}>{copy.columns.status}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((d) => (
            <tr key={d.key} className="border-b align-top last:border-b-0">
              <th scope="row" className={cn("break-words px-4 py-3 text-left font-medium", d.status === "n" ? "text-muted-foreground" : "text-foreground")}>
                {d.name}
              </th>
              <td className="px-4 py-3 text-muted-foreground">
                <FamilyModule device={d} />
              </td>
              <td className="break-words px-4 py-3 text-muted-foreground">{transportLabel(d.transport)}</td>
              <td className="px-4 py-3 text-muted-foreground">
                <Ids ids={d.ids} />
              </td>
              <td className="px-4 py-3">
                <StatusBadge status={d.status} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function ResultCards({ rows }: { rows: Device[] }) {
  return (
    <ul className="space-y-3 lg:hidden" aria-label={copy.tableCaption}>
      {rows.map((d) => (
        <li key={d.key} className="rounded-card border bg-card p-4">
          <div className="flex flex-col gap-2">
            <p className={cn("break-words text-body font-medium", d.status === "n" ? "text-muted-foreground" : "text-foreground")}>{d.name}</p>
            <StatusBadge status={d.status} className="self-start" />
          </div>
          <dl className="mt-3 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1.5 text-small">
            <dt className="text-subtle-foreground">{copy.columns.family}</dt>
            <dd className="min-w-0 text-muted-foreground">
              <FamilyModule device={d} />
            </dd>
            <dt className="text-subtle-foreground">{copy.columns.connection}</dt>
            <dd className="min-w-0 text-muted-foreground">{transportLabel(d.transport)}</dd>
            <dt className="text-subtle-foreground">{copy.columns.ids}</dt>
            <dd className="min-w-0 text-muted-foreground">
              <Ids ids={d.ids} />
            </dd>
          </dl>
        </li>
      ))}
    </ul>
  )
}

export function StatusLegend() {
  const items: { status: DeviceStatus; help: string }[] = [
    { status: "s", help: devicesPage.statusHelp.supported },
    { status: "b", help: devicesPage.statusHelp.blocked },
    { status: "n", help: devicesPage.statusHelp.missing },
  ]
  return (
    <div className="mt-10">
      <h3 className="text-h3">{copy.legendTitle}</h3>
      <dl className="mt-4 grid gap-3 md:grid-cols-3">
        {items.map((item) => (
          <div key={item.status} className="rounded-card border p-4">
            <dt>
              <StatusBadge status={item.status} />
            </dt>
            <dd className="mt-2 text-small text-muted-foreground">{item.help}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}

export function DeviceSearch() {
  // Initial state is constant so the server render and the first client render match.
  const [query, setQuery] = useState("")
  const [status, setStatus] = useState<StatusFilter>("supported")
  const [transport, setTransport] = useState("all")
  const [limit, setLimit] = useState(PAGE_SIZE)
  const urlReady = useRef(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const inputId = useId()
  const countId = useId()

  // Read ?q= after mount (browser only).
  useEffect(() => {
    const q = new URLSearchParams(window.location.search).get("q")
    if (q) setQuery(q)
    urlReady.current = true
  }, [])

  // Keep ?q= in sync, debounced, without adding history entries.
  useEffect(() => {
    if (!urlReady.current) return
    const timer = window.setTimeout(() => {
      const url = new URL(window.location.href)
      const q = query.trim()
      if (q) url.searchParams.set("q", q)
      else url.searchParams.delete("q")
      if (url.href !== window.location.href) window.history.replaceState(window.history.state, "", url)
    }, 300)
    return () => window.clearTimeout(timer)
  }, [query])

  // Filtering 2,000 rows is quick, but deferring keeps typing smooth on slow phones.
  const deferredQuery = useDeferredValue(query)
  const words = useMemo(() => queryWords(deferredQuery), [deferredQuery])
  const queryMatches = useMemo(() => matchQuery(words, transport), [words, transport])
  const results = useMemo(() => queryMatches.filter((d) => matchesStatus(d, status)), [queryMatches, status])
  const otherCount = queryMatches.length - results.length

  useEffect(() => setLimit(PAGE_SIZE), [words, status, transport])

  const shown = results.slice(0, limit)
  const remaining = results.length - shown.length

  const clear = () => {
    setQuery("")
    inputRef.current?.focus()
  }

  return (
    <div>
      <h2 className="sr-only">{copy.heading}</h2>
      <form
        role="search"
        onSubmit={(event) => {
          event.preventDefault()
          inputRef.current?.blur()
        }}
      >
        <label htmlFor={inputId} className="text-small font-medium text-foreground">
          {devicesPage.searchLabel}
        </label>
        <div className="relative mt-2 max-w-2xl">
          <Search aria-hidden className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-subtle-foreground" />
          <Input
            ref={inputRef}
            id={inputId}
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape" && query) {
                event.preventDefault()
                setQuery("")
              }
            }}
            placeholder={devicesPage.placeholder}
            autoComplete="off"
            spellCheck={false}
            enterKeyHint="search"
            aria-describedby={countId}
            className="h-12 rounded-lg bg-card pl-10 pr-12 text-body md:text-body [&::-webkit-search-cancel-button]:appearance-none"
          />
          {query && (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={clear}
              aria-label={copy.clear}
              className="absolute right-1 top-1/2 size-10 -translate-y-1/2 text-muted-foreground"
            >
              <X aria-hidden />
            </Button>
          )}
        </div>
      </form>

      <div className="mt-6 flex flex-col gap-4 lg:flex-row lg:items-start lg:gap-8">
        <div role="group" aria-label={copy.statusGroup} className="flex flex-wrap gap-2">
          {copy.statusFilters.map((f) => (
            <Chip key={f.id} pressed={status === f.id} onClick={() => setStatus(f.id)}>
              {f.label}
            </Chip>
          ))}
        </div>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:gap-3">
          <span aria-hidden className="pt-2.5 text-caption uppercase tracking-wider text-subtle-foreground md:pt-2">
            {copy.transportGroup}
          </span>
          <div role="group" aria-label={copy.transportGroup} className="flex flex-wrap gap-2">
            <Chip pressed={transport === "all"} onClick={() => setTransport("all")}>
              {copy.transportAll}
            </Chip>
            {transports.map((t) => (
              <Chip key={t} pressed={transport === t} onClick={() => setTransport(t)}>
                {transportLabel(t)}
              </Chip>
            ))}
          </div>
        </div>
      </div>

      <div className="mt-8 flex flex-wrap items-baseline justify-between gap-x-6 gap-y-2 border-t pt-6">
        <p id={countId} aria-live="polite" aria-atomic="true" className="text-body font-medium text-foreground">
          {copy.count(results.length)}
        </p>
        {results.length > 0 && <p className="text-small text-subtle-foreground">{copy.showing(shown.length, results.length)}</p>}
      </div>

      <div className="mt-4">
        {results.length === 0 ? (
          <div className="rounded-card border bg-card px-6 py-10 text-center">
            <p className="mx-auto max-w-md text-body text-muted-foreground">{devicesPage.empty}</p>
            {otherCount > 0 && (
              <div className="mt-5 space-y-3">
                <p className="text-small text-subtle-foreground">{copy.otherMatches(otherCount)}</p>
                <Button variant="outline" onClick={() => setStatus("all")}>
                  {copy.showAll}
                </Button>
              </div>
            )}
          </div>
        ) : (
          <>
            <ResultsTable rows={shown} />
            <ResultCards rows={shown} />
            {remaining > 0 && (
              <div className="mt-6 flex justify-center">
                <Button variant="outline" size="lg" onClick={() => setLimit((n) => n + PAGE_SIZE)}>
                  {copy.showMore(Math.min(PAGE_SIZE, remaining))}
                </Button>
              </div>
            )}
            {status !== "all" && otherCount > 0 && words.length > 0 && (
              <p className="mt-6 text-center text-small text-subtle-foreground">
                {copy.otherMatches(otherCount)}{" "}
                <button
                  type="button"
                  onClick={() => setStatus("all")}
                  className="rounded-sm font-medium text-brand-foreground underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  {copy.showAll}
                </button>
              </p>
            )}
          </>
        )}
      </div>

      <StatusLegend />
    </div>
  )
}
