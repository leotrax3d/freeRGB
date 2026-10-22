// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "LenovoControllers");
const files = [];
const walk = (d) => { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const f = path.join(d, e.name); e.isDirectory() ? walk(f) : /\.(cpp|h)$/.test(e.name) && files.push(f); } };
walk(family);
const source = files.map((f) => fs.readFileSync(f, "utf8")).join("\n");
const values = new Map();
for (const m of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) values.set(m[1], m[2]);
for (const m of source.matchAll(/^\s*([A-Z][A-Z0-9_]+)\s*=\s*(0x[0-9A-Fa-f]+)/gm)) values.set(m[1], m[2]);
const resolve = (x) => values.get(x) ?? null;
const detectors = [];
const re = /REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*([A-Z0-9_]+)/g;
for (const m of source.matchAll(re)) { const vid = resolve(m[3]), pid = resolve(m[4]); if (vid && pid) detectors.push({ name: m[1], handler: m[2], vid, pid }); }
if (detectors.length !== 24) throw new Error(`expected 24 Lenovo detectors, found ${detectors.length}`);
const meta = (d) => {
  if (d.handler.includes("4Zone")) return ["four-zone", 4];
  if (d.handler.includes("K500")) return ["k500", 104];
  if (d.handler.includes("K510")) return ["k510", 1];
  if (d.handler.includes("M300")) return ["m300", 1];
  if (d.handler.includes("Gen7And8")) {
    if (d.name.includes("Gen 10")) return ["gen7-8", 24];
    if (d.name.includes("Gen 7") && !d.name.includes("7S")) return ["gen7-8", 120];
    if (d.name.includes("Gen 8") || d.name.includes("Gen 9")) return ["gen7-8", 111];
    return ["gen7-8", 101];
  }
  if (d.name.includes("Y740")) return ["legacy", d.name.includes("Y740") ? 108 : 108];
  if (d.name.includes("Y750")) return ["legacy", 148];
  return ["legacy", d.name.includes("Y760") ? 351 : 145];
};
const devices = detectors.map((d) => { const [protocol, leds] = meta(d); return { ...d, protocol, leds }; });
const out = ["// SPDX-License-Identifier: GPL-2.0-or-later", `// Generated from OpenRGB ${commit}.`, "use super::table::{Detector, Device};", "pub static DETECTORS: &[Detector] = &[", ...devices.map((d) => `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid}, protocol: ${JSON.stringify(d.protocol)}, leds: ${d.leds} },`), "];", "pub static DEVICES: &[Device] = &[", ...devices.map((d) => `    Device { vid: ${d.vid}, pid: ${d.pid}, protocol: ${JSON.stringify(d.protocol)}, leds: ${d.leds} },`), "];", ""];
fs.writeFileSync("modules/lenovo/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} Lenovo detectors`);
