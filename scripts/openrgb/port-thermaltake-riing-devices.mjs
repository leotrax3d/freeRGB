// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";

const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "ThermaltakeRiingController");
const files = [];
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full);
    else if (/\.(cpp|h)$/.test(entry.name)) files.push(full);
  }
}
walk(family);
const source = files.map((file) => fs.readFileSync(file, "utf8")).join("\n");
const constants = new Map();
for (const m of source.matchAll(/^\s*#define\s+(THERMALTAKE_RIING_VID)\s+(0x[0-9A-Fa-f]+)/gm)) constants.set(m[1], m[2]);
const vid = constants.get("THERMALTAKE_RIING_VID") ?? "0x264a";
const detectors = [];
const re = /REGISTER_HID_DETECTOR\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*THERMALTAKE_RIING_VID\s*,\s*(0x[0-9A-Fa-f]+)\s*\)/g;
for (const m of source.matchAll(re)) detectors.push({ name: m[1], handler: m[2], vid, pid: m[3] });
if (detectors.length !== 51) throw new Error(`expected 51 detectors, found ${detectors.length}`);
const lines = [
  "// SPDX-License-Identifier: GPL-2.0-or-later",
  `// Generated from OpenRGB ${commit} by scripts/openrgb/port-thermaltake-riing-devices.mjs.`,
  "use super::table::Detector;",
  "pub static DETECTORS: &[Detector] = &[",
  ...detectors.map((d) => `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid} },`),
  "];",
  "",
];
const out = path.join("modules", "thermaltake-riing", "src", "devices_table.rs");
fs.writeFileSync(out, lines.join("\n"));
console.log(`generated ${detectors.length} Thermaltake Riing HID detectors`);
