import { useMemo, useState } from "react"
import { Search } from "lucide-react"
// Read-only consumers: these files are generated elsewhere and never edited by the website.
import openrgb from "../../../docs/openrgb-compatibility.json"
import fans from "../../../docs/fan-coverage.json"

type RgbDevice = { name: string; family: string; status: string; implementation: string }
type Chip = { chip: string; name: string; vendor: string; controls: number; fans: number; tier: string; status: string }

const rgbDevices = openrgb.devices as RgbDevice[]
const chips = fans.chips as Chip[]

const RGB_LABEL: Record<string, string> = {
  implemented: "Implemented (simulated tests)",
  "hardware-verified": "Hardware verified",
  "not-implemented": "Not yet ported",
  "blocked:smbus-transport": "Blocked on SMBus transport",
}
const TIER_LABEL: Record<string, string> = {
  verified: "Verified chip family",
  "lhm-supported": "Supported via LibreHardwareMonitor",
  "read-only": "Read-only (monitoring only)",
}
const STATUS_LABEL: Record<string, string> = {
  "hardware-verified": "hardware verified",
  simulated: "simulated only",
  unsupported: "no fan control",
}

function count<T>(items: T[], key: (item: T) => string): [string, number][] {
  const map = new Map<string, number>()
  for (const item of items) map.set(key(item), (map.get(key(item)) ?? 0) + 1)
  return [...map].sort((a, b) => b[1] - a[1])
}

function Stat({ label, value }: { label: string; value: number | string }) {
  return <div className="cov-stat"><b>{value}</b><small>{label}</small></div>
}

export function RgbCoverage() {
  const byStatus = useMemo(() => count(rgbDevices, (d) => d.status), [])
  const families = useMemo(() => count(rgbDevices.filter((d) => d.status !== "not-implemented"), (d) => d.family).slice(0, 12), [])
  return <div className="cov">
    <div className="cov-stats">
      <Stat label="Devices known to OpenRGB" value={rgbDevices.length} />
      {byStatus.map(([status, n]) => <Stat key={status} label={RGB_LABEL[status] ?? status} value={n} />)}
    </div>
    <p className="section-note">Counts come from the generated OpenRGB comparison for commit {openrgb.openrgb_commit.slice(0, 8)}. "Implemented" means the protocol is ported and covered by simulated tests; it is not a promise that every unit works until confirmed on real hardware. Use "Check your hardware" for a specific model.</p>
    <div className="device-list">
      <div className="device-head"><span>Largest ported controller families</span><span>Devices</span></div>
      {families.map(([name, n]) => <article className="device-row" key={name}><div><b>{name}</b></div><p>{n}</p></article>)}
    </div>
  </div>
}

export function FanCoverage() {
  const [query, setQuery] = useState("")
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase()
    const list = q ? chips.filter((c) => `${c.chip} ${c.name} ${c.vendor}`.toLowerCase().includes(q)) : chips
    return list.slice(0, 40)
  }, [query])
  const s = fans.summary
  return <div className="cov">
    <div className="cov-stats">
      <Stat label="Fan controller chips" value={s.chips} />
      <Stat label="With fan control" value={s.controllable} />
      <Stat label="Hardware verified" value={s.status["hardware-verified"]} />
      <Stat label="Simulated only" value={s.status.simulated} />
      <Stat label="Read-only" value={s.status.unsupported} />
      <Stat label="Supported board EC models" value={s.ec_boards} />
    </div>
    <p className="section-note">Fan control is gated behind a safety acknowledgement and guided channel setup. Almost all chips are covered by simulation and reference behaviour; very few are confirmed on real hardware. Source: LibreHardwareMonitor {fans.lhm_version}.</p>
    <label className="check-input"><Search size={16} /><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search a chip, e.g. NCT6798 or ITE" aria-label="Search fan controller chips" /></label>
    <div className="device-list">
      <div className="device-head"><span>Chip</span><span>Support</span></div>
      {shown.map((c) => <article className="device-row" key={c.chip}><div><b>{c.name}</b><code>{c.controls} controls · {c.fans} fan inputs</code></div><p>{TIER_LABEL[c.tier] ?? c.tier} — {STATUS_LABEL[c.status] ?? c.status}</p></article>)}
      {shown.length === 0 && <article className="device-row"><div><b>No chip found</b></div><p>freeRGB reads the exact chip at runtime under Diagnostics.</p></article>}
    </div>
  </div>
}
