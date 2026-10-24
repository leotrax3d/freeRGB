// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "NollieController");
const files = fs.readdirSync(family).filter((f) => /\.(cpp|h)$/.test(f)).map((f) => path.join(family, f));
const source = files.map((f) => fs.readFileSync(f, "utf8")).join("\n");
const values = new Map();
for (const m of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) values.set(m[1], m[2]);
const detectors = [];
for (const m of source.matchAll(/REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*([A-Z0-9_]+)/g)) {
  if (values.has(m[3]) && values.has(m[4])) detectors.push({ name: m[1], handler: m[2], vid: values.get(m[3]), pid: values.get(m[4]) });
}
if (detectors.length !== 16) throw new Error(`expected 16 Nollie detectors, found ${detectors.length}`);
const meta = (d) => {
  const n = d.name;
  if (n.includes("32")) return [32, 256, "high"];
  if (n.includes("16")) return [16, 256, "high"];
  if (n.includes("28 L")) return [8, 525, "fixed"];
  if (n.includes("8")) return [8, 126, "fixed"];
  if (n.includes("28")) return [1, 42, "fixed"];
  return [1, 630, "fixed"];
};
const out = ["// SPDX-License-Identifier: GPL-2.0-or-later", `// Generated from OpenRGB ${commit}.`, "use super::table::{Detector,Device};", "pub static DETECTORS:&[Detector]= &[", ...detectors.map((d) => { const [channels, leds, protocol] = meta(d); return `    Detector{name:${JSON.stringify(d.name)},handler:${JSON.stringify(d.handler)},vid:${d.vid},pid:${d.pid},channels:${channels},leds:${leds},protocol:${JSON.stringify(protocol)}},`; }), "];", "pub static DEVICES:&[Device]= &[", ...detectors.map((d) => { const [channels, leds, protocol] = meta(d); return `    Device{vid:${d.vid},pid:${d.pid},channels:${channels},leds:${leds},protocol:${JSON.stringify(protocol)}},`; }), "];", ""];
fs.writeFileSync("modules/nollie/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} Nollie detectors`);
