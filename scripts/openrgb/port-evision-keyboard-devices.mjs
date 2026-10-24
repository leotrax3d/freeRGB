// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "EVisionKeyboardController");
const files = [];
function walk(dir) { for (const e of fs.readdirSync(dir, { withFileTypes: true })) { const p = path.join(dir, e.name); if (e.isDirectory()) walk(p); else if (/\.(cpp|h)$/.test(e.name)) files.push(p); } }
walk(family);
const source = files.map((f) => fs.readFileSync(f, "utf8")).join("\n");
const values = new Map();
for (const m of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) values.set(m[1], m[2]);
const detectors = [];
for (const m of source.matchAll(/REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*([A-Z0-9_]+)/g)) {
  if (values.has(m[3]) && values.has(m[4])) detectors.push({ name: m[1], handler: m[2], vid: values.get(m[3]), pid: values.get(m[4]) });
}
if (detectors.length !== 15) throw new Error(`expected 15 EVision detectors, found ${detectors.length}`);
const out = ["// SPDX-License-Identifier: GPL-2.0-or-later", `// Generated from OpenRGB ${commit}.`, "use super::table::{Detector,Device};", "pub static DETECTORS:&[Detector]= &[", ...detectors.map((d) => `    Detector{name:${JSON.stringify(d.name)},handler:${JSON.stringify(d.handler)},vid:${d.vid},pid:${d.pid},v2:${d.handler.includes("V2") || d.handler.includes("Endorfy")}},`), "];", "pub static DEVICES:&[Device]= &[", ...detectors.map((d) => `    Device{vid:${d.vid},pid:${d.pid},v2:${d.handler.includes("V2") || d.handler.includes("Endorfy")}},`), "];", ""];
fs.writeFileSync("modules/evision-keyboard/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} EVision detectors`);
