import { useMemo, useState } from "react"
import { Cpu, Search } from "lucide-react"
import { checkBoard, looksLikeBoard } from "./boardCheck"
import database from "../data/hardware.json"

type Status = "builtin" | "module" | "openrgb"
type Device = { name: string; ids: string[]; category: string; status: Status; source: string }

const devices = database.devices as Device[]
const boards = database.boards as { vendor: string; name: string }[]
const LIMIT = 40

const STATUS: Record<Status, { label: string; tone: string; text: string }> = {
  builtin: { label: "Supported", tone: "ok", text: "Built into freeRGB, detected automatically." },
  module: { label: "Available as module", tone: "module", text: "Install the hardware module from the Devices page. Ported from OpenRGB; experimental until confirmed on real hardware." },
  openrgb: { label: "Not yet supported", tone: "no", text: "Known to OpenRGB, not ported to freeRGB yet." },
}

/** Every query word must appear in the name or USB ID. */
export function search(query: string): Device[] {
  const words = query.toLowerCase().replace(/[^a-z0-9: ]+/g, " ").split(/\s+/).filter(Boolean)
  if (!words.length) return []
  const rank: Record<Status, number> = { builtin: 0, module: 1, openrgb: 2 }
  return devices
    .filter((d) => {
      const hay = `${d.name} ${d.ids.join(" ")}`.toLowerCase()
      return words.every((w) => hay.includes(w))
    })
    .sort((a, b) => rank[a.status] - rank[b.status] || a.name.length - b.name.length)
}

function MainboardCard({ query }: { query: string }) {
  const verdict = useMemo(() => checkBoard(query, boards), [query])
  if (!verdict) return null
  const sub = [verdict.brand !== "Unknown" && verdict.brand, verdict.chipset && `${verdict.platform} ${verdict.chipset}`].filter(Boolean).join(" · ")
  return <article className="check-board">
    <div className="check-board-title"><Cpu size={16} /><b>{verdict.title}</b>{sub && <span>{sub}</span>}</div>
    <dl>{verdict.lines.map((l) => [
      <dt key={`${l.label}-t`}>{l.label}</dt>,
      <dd key={`${l.label}-d`}><span className={`check-badge ${l.tone}`}>{l.badge}</span> {l.text}</dd>,
    ])}</dl>
    {verdict.brand === "Unknown" && <p className="section-note">Add the brand (ASUS, MSI, Gigabyte, ASRock) for a model-specific answer.</p>}
  </article>
}

export function HardwareCheck() {
  const [query, setQuery] = useState("")
  const results = useMemo(() => search(query), [query])
  const counts = useMemo(() => ({
    builtin: devices.filter((d) => d.status === "builtin").length,
    module: devices.filter((d) => d.status === "module").length,
  }), [])
  return <div className="check">
    <label className="check-input">
      <Search size={16} />
      <input type="search" value={query} onChange={(e) => setQuery(e.target.value)} autoComplete="off" spellCheck={false}
        placeholder="Exact model, e.g. ASUS TUF GAMING B650-PLUS WIFI, MSI MAG X870 TOMAHAWK, Corsair K70 RGB PRO, 1532:0043" aria-label="Search your hardware" />
    </label>
    <p className="section-note">{counts.builtin + counts.module} devices supported or available as module, plus the OpenRGB catalogue for everything not yet ported. Motherboards: type the full model name as printed on the box. Tip: the USB ID (Windows Device Manager → Hardware IDs, or <code>lsusb</code>) is the most exact search.</p>
    <MainboardCard query={query} />
    {query.trim() && !(looksLikeBoard(query) && results.length === 0) && <div className="device-list">
      <div className="device-head"><span>Device</span><span>Status in freeRGB</span></div>
      {results.slice(0, LIMIT).map((d) => <article className="device-row" key={`${d.status}|${d.name}`}>
        <div><b>{d.name}</b><code>{[d.category, d.ids.length ? d.ids.join(", ") : d.source].join(" · ")}</code></div>
        <p><span className={`check-badge ${STATUS[d.status].tone}`}>{STATUS[d.status].label}</span> {STATUS[d.status].text}</p>
      </article>)}
      {results.length === 0 && <article className="device-row"><div><b>No match</b></div><p>Not in freeRGB or OpenRGB. Some models have no controllable lighting at all. Check the spelling or search by USB ID, or request it with a <a href="https://github.com/leotrax3d/FreeRGB/issues/new?template=hardware-report.md" target="_blank" rel="noreferrer">hardware report</a>.</p></article>}
      {results.length > LIMIT && <article className="device-row"><div><b>{results.length - LIMIT} more</b></div><p>Add words (model, revision) to narrow the list.</p></article>}
    </div>}
  </div>
}
