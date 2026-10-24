// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";

const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "CherryKeyboardController");
const files = fs.readdirSync(family).filter((f) => /\.(cpp|h)$/.test(f)).map((f) => path.join(family, f));
const source = files.map((file) => fs.readFileSync(file, "utf8")).join("\n");
const constants = new Map();
for (const match of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) constants.set(match[1], match[2]);
const detectors = [];
const pattern = /REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*([A-Z0-9_]+)/g;
for (const match of source.matchAll(pattern)) {
  const vid = constants.get(match[3]);
  const pid = constants.get(match[4]);
  if (vid && pid) detectors.push({ name: match[1], handler: match[2], vid, pid });
}
if (detectors.length !== 25) throw new Error(`expected 25 Cherry detectors, found ${detectors.length}`);
const out = [
  "// SPDX-License-Identifier: GPL-2.0-or-later",
  `// Generated from OpenRGB ${commit}.`,
  "use super::table::Detector;",
  "pub static DETECTORS: &[Detector] = &[",
  ...detectors.map((d) => `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid} },`),
  "];",
  "",
];
fs.writeFileSync("modules/cherry/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} Cherry detectors`);
