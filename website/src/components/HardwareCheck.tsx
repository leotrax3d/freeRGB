import { useMemo, useState } from "react"
import { Search } from "lucide-react"
import database from "../data/hardware.json"

type Status = "builtin" | "module"
type Device = { name: string; ids: string[]; category: string; status: Status; source: string }

type CatalogueDevice = Omit<Device, "status"> & { status: Status | "openrgb" }
const devices = (database.devices as CatalogueDevice[]).filter((device): device is Device => device.status !== "openrgb")
const LIMIT = 40

const STATUS: Record<Status, { label: string; tone: string; text: string }> = {
  builtin: { label: "Built in", tone: "ok", text: "Detected automatically." },
  module: { label: "Module", tone: "module", text: "Install from the Devices page." },
}

/** Every query word must appear in the name or USB ID. */
export function search(query: string): Device[] {
  const words = query.toLowerCase().replace(/[^a-z0-9: ]+/g, " ").split(/\s+/).filter(Boolean)
  if (!words.length) return []
  const rank: Record<Status, number> = { builtin: 0, module: 1 }
  return devices
    .filter((d) => {
      const hay = `${d.name} ${d.ids.join(" ")}`.toLowerCase()
      return words.every((w) => hay.includes(w))
    })
    .sort((a, b) => rank[a.status] - rank[b.status] || a.name.length - b.name.length)
}

export function HardwareCheck() {
  const [query, setQuery] = useState("")
  const results = useMemo(() => search(query), [query])
  return <div className="check">
    <label className="check-input">
      <Search size={16} />
      <input type="search" value={query} onChange={(e) => setQuery(e.target.value)} autoComplete="off" spellCheck={false}
        placeholder="Search a model or USB ID, e.g. Corsair K70 or 1532:0043" aria-label="Search your hardware" />
    </label>
    <div className="check-examples">{["Corsair K70", "Razer", "ASUS"].map((model) => <button type="button" key={model} onClick={() => setQuery(model)}>{model}</button>)}</div>
    <p className="search-result-count" role="status" aria-live="polite">{query.trim() ? `${results.length} ${results.length === 1 ? "match" : "matches"}` : ""}</p>
    {query.trim() && <div className="device-list">
      <div className="device-head"><span>Device</span><span>Status in freeRGB</span></div>
      {results.slice(0, LIMIT).map((d) => <article className="device-row" key={`${d.status}|${d.name}`}>
        <div><b>{d.name}</b><code>{[d.category, d.ids.length ? d.ids.join(", ") : d.source].join(" · ")}</code></div>
        <p><span className={`check-badge ${STATUS[d.status].tone}`}>{STATUS[d.status].label}</span> {STATUS[d.status].text}</p>
      </article>)}
      {results.length === 0 && <article className="device-row"><div><b>No match</b></div><p>Try the full model name or USB ID.</p></article>}
      {results.length > LIMIT && <article className="device-row"><div><b>{results.length - LIMIT} more</b></div><p>Add words (model, revision) to narrow the list.</p></article>}
    </div>}
  </div>
}
