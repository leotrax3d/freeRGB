import devicesData from "virtual:devices"

export type DeviceStatus = "s" | "b" | "n"
export type StatusFilter = "supported" | "unsupported" | "all"

export type Device = {
  key: string
  name: string
  family: string
  transport: string
  ids: string
  status: DeviceStatus
  module: string
  /** Lowercase search text: name, family, ids (with ":" , " " and no separator) and module. */
  hay: string
}

/** Display names for the transport values in docs/openrgb-compatibility.json. Unknown values show as-is. */
const TRANSPORT_LABELS: Record<string, string> = {
  "USB HID": "USB HID",
  "I2C/PCI (GPU)": "GPU (I2C/PCI)",
  "SMBus DRAM": "RAM (SMBus)",
  "SMBus/I2C": "SMBus/I2C",
  "USB serial": "USB serial",
  "other (network/serial/OS)": "Other (network, serial, OS)",
}

export const transportLabel = (transport: string) => TRANSPORT_LABELS[transport] ?? transport

const STATUS_ORDER: Record<DeviceStatus, number> = { s: 0, b: 1, n: 2 }

// Plain code-unit comparison keeps the order identical in Node (SSR) and every browser.
const compare = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0)

export const devices: Device[] = devicesData.rows
  .map(([name, family, transport, ids, status, module], index) => {
    const lowerIds = ids.toLowerCase()
    return {
      key: `${index}`,
      name,
      family,
      transport,
      ids,
      status,
      module,
      hay: [name, family, lowerIds, lowerIds.replace(/:/g, " "), lowerIds.replace(/:/g, ""), module].join(" ").toLowerCase(),
    }
  })
  .sort((a, b) => STATUS_ORDER[a.status] - STATUS_ORDER[b.status] || compare(a.name.toLowerCase(), b.name.toLowerCase()) || compare(a.ids, b.ids))

/** Transports ordered by how many devices use them. */
export const transports: string[] = (() => {
  const counts = new Map<string, number>()
  for (const d of devices) counts.set(d.transport, (counts.get(d.transport) ?? 0) + 1)
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || compare(a[0], b[0])).map(([t]) => t)
})()

export function queryWords(query: string) {
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => word.replace(/^0x(?=[0-9a-f])/, ""))
}

export function matchesStatus(device: Device, filter: StatusFilter) {
  if (filter === "all") return true
  return filter === "supported" ? device.status === "s" : device.status !== "s"
}

/** Devices that match every query word and the connection filter, ignoring the status filter. */
export function matchQuery(words: string[], transport: string) {
  return devices.filter((d) => (transport === "all" || d.transport === transport) && words.every((w) => d.hay.includes(w)))
}

/** Thousands separator without Intl, so server and browser output are identical. */
export const formatCount = (n: number) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ",")
