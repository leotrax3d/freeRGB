// SPDX-License-Identifier: MIT OR Apache-2.0
// Builds website/src/data/hardware.json, the database behind the website's
// "Check your hardware" search. Only facts are extracted (names, USB IDs);
// no OpenRGB code is copied.
//
// Usage: node scripts/website/build-hardware-db.mjs [<openrgb-checkout> [<librehardwaremonitor-checkout>]]
//   Without a checkout, the OpenRGB "not yet supported" tier is kept from the
//   existing JSON so the script can run offline.
import { existsSync, readdirSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));
const output = join(root, "website/src/data/hardware.json");
const openrgb = process.argv[2];
const hex = (n) => Number(n).toString(16).padStart(4, "0");
const id = (vid, pid) => `${hex(vid)}:${hex(pid)}`;

/** @type {Map<string, {name:string, id?:string, category:string, status:string, source:string}>} */
const entries = new Map();
const add = (entry) => {
  const key = `${entry.id ?? ""}|${entry.name.toLowerCase()}`;
  const previous = entries.get(key);
  // Better status wins: builtin > module > openrgb.
  const rank = { builtin: 0, module: 1, openrgb: 2 };
  if (!previous || rank[entry.status] < rank[previous.status]) entries.set(key, entry);
};

const category = (name) => {
  const n = name.toLowerCase();
  if (/nzxt kraken|\baio\b|cooler|hydro|pump|waterforce|liquid|capellix/.test(n)) return "Cooler / AIO";
  if (/motherboard|mainboard|mystic|aura (usb )?mb|polychrome|fusion|\bms-\d/.test(n)) return "Motherboard";
  if (/keyboard|keypad|\bk\d{2,3}\b|apex|alloy|vulcan|blackwidow|huntsman|ornata|cynosa|g\d{3} .*mech/.test(n)) return "Keyboard";
  if (/mouse|mice|aerox|prime|deathadder|basilisk|viper|naga|rival|sensei|kone|pulsefire|\bg\d{3}\b/.test(n)) return "Mouse";
  if (/headset|kraken|arctis|cloud|headphone/.test(n)) return "Headset";
  if (/mouse ?pad|mousemat|mat\b|goliathus|firefly|qck/.test(n)) return "Mousepad";
  if (/\bfan|riing|hub|commander|lighting node|controller|argb|led strip|smart device|hue|nollie|uni ?fan/.test(n)) return "Fans & controllers";
  if (/geforce|radeon|rtx|gtx|\brx ?\d|gpu|graphics/.test(n)) return "Graphics card";
  if (/dram|\bddr|vengeance|trident|fury|dominator|memory|ram\b/.test(n)) return "Memory (RAM)";
  if (/monitor|display|screen/.test(n)) return "Monitor";
  if (/laptop|notebook|legion|aorus .*laptop/.test(n)) return "Laptop";
  return "Other";
};

// 1. Built-in daemon adapters (crates/freergb-protocols registry + docs).
const builtin = [
  ["CORSAIR K70 CORE RGB", 0x1b1c, 0x1bfd],
  ["CORSAIR K70 CORE RGB", 0x1b1c, 0x1bff],
  ["CORSAIR iCUE LINK System Hub (RX/LX fans, Titan RX AIO)", 0x1b1c, 0x0c3f],
  ...[0x18f3, 0x1939, 0x19af, 0x1aa6, 0x1bed].map((pid) => ["ASUS Aura motherboard RGB/ARGB (USB)", 0x0b05, pid]),
  ...[0x5702, 0x8297].map((pid) => ["Gigabyte RGB Fusion 2 motherboard ARGB (USB)", 0x048d, pid]),
  ...[0x01a2, 0x01a6].map((pid) => ["ASRock Polychrome motherboard RGB/ARGB (USB)", 0x26ce, pid]),
];
for (const [name, vid, pid] of builtin) add({ name, id: id(vid, pid), category: category(name), status: "builtin", source: "freeRGB" });

// 2. Installable hardware modules (ported OpenRGB families).
for (const module of readdirSync(join(root, "modules"))) {
  const table = join(root, "modules", module, "src/devices_table.rs");
  if (!existsSync(table)) continue;
  // Default VID for tables without one per device ("1532=Razer" or {vid}).
  const info = JSON.parse(readFileSync(join(root, "modules", module, "module-info.json"), "utf8"));
  const vendors = [info.vendor, ...(info.vendors ?? [])].filter(Boolean);
  const fallback = vendors.map((v) => (typeof v === "string" ? v.split("=")[0] : v.vid)).find((v) => /^[0-9A-Fa-f]{4}$/.test(v ?? ""));
  const text = readFileSync(table, "utf8");
  for (const block of text.split(/\n\s*\w+\s*\{/).slice(1)) {
    const name = block.match(/name:\s*"([^"]+)"/)?.[1];
    const pid = block.match(/pid:\s*(0x[0-9A-Fa-f]+)/)?.[1];
    const vid = block.match(/vid:\s*(0x[0-9A-Fa-f]+)/)?.[1] ?? (fallback && `0x${fallback}`);
    if (!name) continue;
    add({ name, id: pid && vid ? id(vid, pid) : undefined, category: category(name), status: "module", source: module });
  }
}

// 3. Everything else OpenRGB detects: "not yet supported" in freeRGB.
let known = [];
const previousCommit = existsSync(output) ? JSON.parse(readFileSync(output, "utf8")).openrgb_commit : "unknown";
if (openrgb) {
  const files = [];
  const walk = (dir) => {
    for (const e of readdirSync(dir, { withFileTypes: true })) {
      const p = join(dir, e.name);
      if (e.isDirectory()) walk(p);
      else if (/\.(cpp|h)$/.test(e.name)) files.push(p);
    }
  };
  walk(join(openrgb, "Controllers"));
  const defines = new Map();
  const sources = files.map((f) => readFileSync(f, "utf8"));
  for (const s of sources) for (const m of s.matchAll(/#define\s+(\w+)\s+(0x[0-9A-Fa-f]+|\d+)\b/g)) defines.set(m[1], Number(m[2]));
  const value = (token) => (/^(0x[0-9a-f]+|\d+)$/i.test(token) ? Number(token) : defines.get(token));
  for (const s of sources) {
    for (const m of s.matchAll(/REGISTER_([A-Z0-9_]*DETECTOR[A-Z0-9_]*)\s*\(\s*"([^"]+)"\s*,([^;]*)\)\s*;/g)) {
      const [, kind, name, rest] = m;
      const args = rest.split(",").map((a) => a.trim());
      let usb;
      if (kind.includes("HID") || kind.includes("USB")) {
        const [vid, pid] = [value(args[1]), value(args[2])];
        if (vid !== undefined && pid !== undefined) usb = id(vid, pid);
      }
      const bus = kind.includes("I2C") || kind.includes("DRAM") ? "SMBus/I²C" : kind.includes("HID") ? "USB" : "other";
      known.push({ name, id: usb, category: category(name), status: "openrgb", source: bus });
    }
  }
} else if (existsSync(output)) {
  known = JSON.parse(readFileSync(output, "utf8")).devices.filter((d) => d.status === "openrgb").flatMap((d) => (d.ids.length ? d.ids : [undefined]).map((id) => ({ ...d, id, ids: undefined })));
}
for (const entry of known) add(entry);

// A USB ID covered by a better tier is not listed again under a worse one.
const best = new Map();
for (const e of entries.values()) {
  if (!e.id) continue;
  const rank = { builtin: 0, module: 1, openrgb: 2 }[e.status];
  if (!best.has(e.id) || rank < best.get(e.id)) best.set(e.id, rank);
}
const kept = [...entries.values()].filter((e) => !e.id || { builtin: 0, module: 1, openrgb: 2 }[e.status] === best.get(e.id));
// One row per device: identical name + status share their USB IDs.
const grouped = new Map();
for (const e of kept) {
  const key = `${e.status}|${e.name.toLowerCase()}`;
  const row = grouped.get(key) ?? { name: e.name, ids: [], category: e.category, status: e.status, source: e.source };
  if (e.id && !row.ids.includes(e.id)) row.ids.push(e.id);
  grouped.set(key, row);
}
const devices = [...grouped.values()].sort((a, b) => a.name.localeCompare(b.name));

// Motherboards LibreHardwareMonitor knows by name (fan header labels).
const lhm = process.argv[3];
let boards = [];
if (lhm) {
  const model = readFileSync(join(lhm, "LibreHardwareMonitorLib/Hardware/Motherboard/Model.cs"), "utf8");
  let vendor = "";
  for (const line of model.slice(model.indexOf("enum Model")).split("\n")) {
    const comment = line.match(/^\s*\/\/\s*(.+?)\s*$/);
    if (comment) vendor = comment[1];
    const m = line.match(/^\s*(\w+),/);
    if (m && m[1] !== "Unknown") boards.push({ vendor, name: m[1].replace(/^_/, "").replace(/_/g, " ") });
  }
} else if (existsSync(output)) {
  boards = JSON.parse(readFileSync(output, "utf8")).boards ?? [];
}
mkdirSync(join(root, "website/src/data"), { recursive: true });
writeFileSync(output, JSON.stringify({
  generated: "scripts/website/build-hardware-db.mjs",
  openrgb_commit: process.env.OPENRGB_COMMIT ?? previousCommit,
  devices,
  boards,
}) + "\n");
const count = (s) => devices.filter((d) => d.status === s).length;
console.log(`hardware.json: ${boards.length} named boards, ${devices.length} entries (builtin ${count("builtin")}, module ${count("module")}, openrgb ${count("openrgb")})`);
