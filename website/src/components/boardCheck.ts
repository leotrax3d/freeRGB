// Motherboard verdicts for a typed model name (brand + chipset + series).
// Chip and controller generations follow the vendors' usual designs; the
// exact chip is confirmed by freeRGB at runtime (Diagnostics → fan hardware).

export type Tone = "ok" | "module" | "warn" | "no"
export type Line = { label: string; tone: Tone; badge: string; text: string }
export type BoardVerdict = { title: string; brand: string; chipset: string | null; platform: "AMD" | "Intel" | null; known: string | null; lines: Line[] }

const AMD = ["A320", "B350", "X370", "B450", "X470", "A520", "B550", "X570", "A620", "B650", "X670", "B840", "B850", "X870", "X399", "TRX40", "TRX50", "WRX80", "WRX90"]
const INTEL = ["H110", "B150", "H170", "Z170", "B250", "H270", "Z270", "H310", "B360", "B365", "H370", "Z370", "Z390", "H410", "B460", "H470", "Z490", "H510", "B560", "H570", "Z590", "H610", "B660", "H670", "Z690", "B760", "H770", "Z790", "H810", "B860", "Z890", "X299", "W680", "W790"]

const BRANDS: [RegExp, string][] = [
  [/\b(asus|rog|tuf|prime|proart|strix|crosshair|maximus)\b/, "ASUS"],
  [/\b(msi|mag|mpg|meg|tomahawk|mortar|carbon|godlike|ms-?\d)/, "MSI"],
  [/\b(gigabyte|aorus)\b/, "Gigabyte"],
  [/\b(asrock|taichi|steel legend|phantom gaming|nova|lightning|riptide)\b/, "ASRock"],
  [/\b(biostar|racing)\b/, "Biostar"],
  [/\b(evga)\b/, "EVGA"],
]

/** 1 = 100/200 series era, 2 = AMD 300 / Intel 300, 3 = AMD 400-500 / Intel 400-500, 4 = AMD 600+ / Intel 600+. */
function generation(chipset: string, platform: "AMD" | "Intel"): number {
  const n = Number(chipset.replace(/\D/g, "").slice(0, 1))
  if (/^(X399|TRX40|X299)$/.test(chipset)) return 2
  if (/^(TRX50|WRX80|WRX90|W680|W790)$/.test(chipset)) return 4
  if (platform === "AMD") return n <= 3 ? 2 : n <= 5 ? 3 : 4
  return n <= 2 ? 1 : n === 3 ? 2 : n <= 5 ? 3 : 4
}

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim()

export function looksLikeBoard(query: string): boolean {
  const q = query.toUpperCase()
  return /\b(MOTHERBOARD|MAINBOARD|MS-?7[0-9A-F]{3})\b/.test(q) || [...AMD, ...INTEL].some((c) => new RegExp(`\\b${c}[EIM]?\\b`).test(q))
}

export function checkBoard(query: string, boards: { vendor: string; name: string }[]): BoardVerdict | null {
  if (!looksLikeBoard(query)) return null
  const q = query.toLowerCase()
  const brand = BRANDS.find(([re]) => re.test(q))?.[1] ?? "Unknown"
  const upper = query.toUpperCase()
  const chipset = [...AMD, ...INTEL].find((c) => new RegExp(`\\b${c}[EIM]?\\b`).test(upper)) ?? null
  const platform = chipset ? (AMD.includes(chipset) ? "AMD" : "Intel") : null
  const gen = chipset && platform ? generation(chipset, platform) : 0
  const nq = norm(query)
  const known = boards.find((b) => {
    const nb = norm(b.name)
    return nb.length > 4 && (nq.includes(nb) || nb.split(" ").every((w) => nq.includes(w)))
  })
  const lines: Line[] = []

  // Fans: which SuperIO chip the vendor normally uses, and what that means per OS.
  let chip = "a Nuvoton, ITE or Fintek SuperIO chip"
  let windows: Line = { label: "Fans · Windows", tone: "ok", badge: "Supported", text: "With the optional mainboard components (LibreHardwareMonitor). Most boards work; a short guided channel test unlocks curves below 100% on chips without audited readback." }
  let linux: Line = { label: "Fans · Linux", tone: "ok", badge: "Supported", text: "Through the kernel driver (nct6775 or it87); every header it exposes can be controlled." }
  if (brand === "ASUS" || brand === "ASRock" || brand === "Biostar") {
    chip = brand === "ASRock" && gen >= 4 ? "Nuvoton NCT6798D/NCT6796D (some Z790/X670 boards: NCT6686D)" : "Nuvoton NCT67xx (e.g. NCT6798D/NCT6799D)"
    windows = { ...windows, badge: "Supported", text: "Nuvoton chips of this family are in freeRGB's verified tier (register readback after every write). Needs the optional mainboard components." }
    linux = { ...linux, text: "Kernel driver nct6775 (in every distribution; nct6683 for NCT6686D boards). Every header the driver exposes can be controlled." }
  } else if (brand === "MSI") {
    chip = gen >= 3 ? "Nuvoton NCT6687D" : "Nuvoton NCT6795D/NCT6797D"
    windows = gen >= 3
      ? { ...windows, tone: "ok", badge: "Supported", text: "NCT6687D runs in the LHM-supported tier: after a one-time guided channel test, curves down to 40% (fan stop with unrestricted control)." }
      : { ...windows, text: "Nuvoton chips of this family are in freeRGB's verified tier. Needs the optional mainboard components." }
    linux = gen >= 3
      ? { label: "Fans · Linux", tone: "warn", badge: "Driver needed", text: "NCT6687D needs the out-of-tree nct6687 driver (github.com/Fred78290/nct6687d); the in-kernel nct6683 driver only reads on most MSI boards." }
      : { ...linux, text: "Kernel driver nct6775; every header it exposes can be controlled." }
  } else if (brand === "Gigabyte") {
    chip = gen >= 3 ? "ITE IT8689E/IT8696E (+ IT87952E on many boards)" : "ITE IT8686E/IT8688E"
    windows = { ...windows, text: "ITE chips run in the LHM-supported tier: after a one-time guided channel test, curves down to 40% (fan stop with unrestricted control)." }
    linux = gen >= 3
      ? { label: "Fans · Linux", tone: "warn", badge: "Driver needed", text: "Newer ITE chips need the out-of-tree it87 driver (github.com/frankcrawford/it87), often with ignore_resource_conflict=1." }
      : { ...linux, text: "Kernel driver it87 (may need ignore_resource_conflict=1)." }
  }
  lines.push({ label: "Fan chip (expected)", tone: "module", badge: "Info", text: `${chip}. freeRGB shows the exact chip after the first start.` }, windows, linux)

  // RGB headers: USB controller generation per vendor.
  let rgb: Line
  const sm = (what: string): Line => ({ label: "RGB/ARGB headers", tone: "no", badge: "Not yet", text: `${what} is driven over SMBus on this generation, which freeRGB does not support yet.` })
  if (!chipset) rgb = { label: "RGB/ARGB headers", tone: "warn", badge: "Unknown", text: "Add the chipset (e.g. B650, Z790) to see the RGB status." }
  else if (brand === "ASUS") rgb = gen >= 3 || (gen === 2 && platform === "Intel")
    ? { label: "RGB/ARGB headers", tone: "ok", badge: "Supported", text: "Aura USB controller, built into freeRGB: 12 V RGB and up to 4 ARGB headers. Budget PRIME boards may have no RGB headers at all." }
    : sm("Aura")
  else if (brand === "Gigabyte") rgb = gen >= 3 || (gen === 2 && platform === "Intel") || chipset === "X470" || chipset === "B450"
    ? { label: "RGB/ARGB headers", tone: "ok", badge: "Supported", text: "RGB Fusion 2 USB (IT5702/IT8297), built into freeRGB: D_LED ARGB headers." }
    : sm("RGB Fusion")
  else if (brand === "MSI") rgb = gen >= 3 || (gen === 2 && platform === "Intel") || chipset === "B450" || chipset === "X470"
    ? { label: "RGB/ARGB headers", tone: "module", badge: "Module", text: "Mystic Light USB: install the MSI motherboard module. Search your MS-7xxx code (printed on the board) to confirm it is in the list." }
    : sm("Mystic Light")
  else if (brand === "ASRock") rgb = gen >= 4
    ? { label: "RGB/ARGB headers", tone: "warn", badge: "Likely", text: "Supported when the board has the Polychrome USB controller (26ce:01a2 in Device Manager); boards without it use SMBus (not supported yet)." }
    : sm("Polychrome")
  else rgb = { label: "RGB/ARGB headers", tone: "no", badge: "Not yet", text: `No ${brand === "Unknown" ? "" : brand + " "}motherboard RGB controller is supported yet.` }
  lines.push(rgb)

  if (known) lines.push({ label: "Board recognized", tone: "ok", badge: "Named headers", text: `LibreHardwareMonitor knows the ${known.vendor} ${known.name}: fan headers appear with their real names (CPU_FAN, CHA_FAN…).` })

  return { title: query.trim().replace(/\s+/g, " "), brand, chipset, platform, known: known ? `${known.vendor} ${known.name}` : null, lines }
}
