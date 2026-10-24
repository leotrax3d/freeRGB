import { useMemo, useState } from "react"
import { Cpu, Search } from "lucide-react"
import database from "../data/hardware.json"

type Status = "builtin" | "module" | "openrgb"
type Device = { name: string; id?: string; category: string; status: Status; source: string }

const devices = database.devices as Device[]
const LIMIT = 40

const STATUS: Record<Status, { label: string; tone: string; text: string }> = {
  builtin: { label: "Supported", tone: "ok", text: "Built into freeRGB, detected automatically." },
  module: { label: "Available as module", tone: "module", text: "Install the hardware module from the Devices page. Ported from OpenRGB; experimental until confirmed on real hardware." },
  openrgb: { label: "Not yet supported", tone: "no", text: "Known to OpenRGB, not ported to freeRGB yet." },
}

const MAINBOARD_RGB: [RegExp, string, string][] = [
  [/\b(asus|rog|tuf|prime|proart|strix)\b/, "ASUS", "Aura USB RGB/ARGB headers: supported on most boards from the 400/500 series on. Older boards use SMBus (not supported)."],
  [/\b(msi|mag|mpg|meg|pro\b)/, "MSI", "Mystic Light: available as module for 120+ boards (MS-7xxx codes). Older boards without a USB controller are not covered."],
  [/\b(gigabyte|aorus)\b/, "Gigabyte", "RGB Fusion 2 USB (IT5702/IT8297): supported on most boards from X470/Z390 on. Older SMBus boards are not covered."],
  [/\b(asrock|phantom|steel legend|taichi)\b/, "ASRock", "Polychrome USB: supported where the board has the USB controller; many ASRock boards use SMBus (not supported)."],
]

/** Board chipset like B650, Z790, X870E, A620, H610. */
const CHIPSET = /\b[abhxz]\d{3}[a-z]?\b/

/** Every query word must appear in the name or USB ID. */
export function search(query: string): Device[] {
  const words = query.toLowerCase().replace(/[^a-z0-9: ]+/g, " ").split(/\s+/).filter(Boolean)
  if (!words.length) return []
  const rank: Record<Status, number> = { builtin: 0, module: 1, openrgb: 2 }
  return devices
    .filter((d) => {
      const hay = `${d.name} ${d.id ?? ""}`.toLowerCase()
      return words.every((w) => hay.includes(w))
    })
    .sort((a, b) => rank[a.status] - rank[b.status] || a.name.length - b.name.length)
}

function MainboardAdvice({ query }: { query: string }) {
  const q = query.toLowerCase()
  if (!isBoard(q)) return null
  const rgb = MAINBOARD_RGB.find(([re]) => re.test(q))
  return <article className="check-board">
    <div className="check-board-title"><Cpu size={16} /><b>Motherboard{rgb ? ` · ${rgb[1]}` : ""}</b></div>
    <dl>
      <dt>Fans on the board</dt>
      <dd><span className="check-badge ok">Supported</span> Windows: through the optional mainboard components (LibreHardwareMonitor), most Nuvoton/ITE/Fintek chips. Linux: every header the kernel driver exposes (nct6775, it87, nct6683, …).</dd>
      <dt>RGB/ARGB headers</dt>
      <dd>{rgb ? rgb[2] : "Include the brand (ASUS, MSI, Gigabyte, ASRock) to see the RGB status."}</dd>
    </dl>
  </article>
}

const isBoard = (q: string) => CHIPSET.test(q.toLowerCase()) || /motherboard|mainboard/.test(q.toLowerCase())

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
        placeholder="e.g. Razer DeathAdder V3, Corsair K70, ASUS TUF B650, 1532:0098" aria-label="Search your hardware" />
    </label>
    <p className="section-note">{counts.builtin + counts.module} devices supported or available as module, plus the OpenRGB catalogue for everything not yet ported. Tip: the USB ID (Windows Device Manager → Hardware IDs, or <code>lsusb</code>) is the most exact search.</p>
    <MainboardAdvice query={query} />
    {query.trim() && !(isBoard(query) && results.length === 0) && <div className="device-list">
      <div className="device-head"><span>Device</span><span>Status in freeRGB</span></div>
      {results.slice(0, LIMIT).map((d) => <article className="device-row" key={`${d.id}|${d.name}`}>
        <div><b>{d.name}</b><code>{[d.category, d.id ?? d.source].join(" · ")}</code></div>
        <p><span className={`check-badge ${STATUS[d.status].tone}`}>{STATUS[d.status].label}</span> {STATUS[d.status].text}</p>
      </article>)}
      {results.length === 0 && <article className="device-row"><div><b>No match</b></div><p>Not in freeRGB or OpenRGB. Some models have no controllable lighting at all. Check the spelling or search by USB ID, or request it with a <a href="https://github.com/leotrax3d/FreeRGB/issues/new?template=hardware-report.md" target="_blank" rel="noreferrer">hardware report</a>.</p></article>}
      {results.length > LIMIT && <article className="device-row"><div><b>{results.length - LIMIT} more</b></div><p>Add words (model, revision) to narrow the list.</p></article>}
    </div>}
  </div>
}
