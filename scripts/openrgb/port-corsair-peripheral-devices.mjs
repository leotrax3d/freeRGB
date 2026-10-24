// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "CorsairPeripheralController");
const files = [];
function walk(dir) { for (const e of fs.readdirSync(dir, { withFileTypes: true })) { const f = path.join(dir, e.name); if (e.isDirectory()) walk(f); else if (/\.(cpp|h)$/.test(e.name)) files.push(f); } }
walk(family);
const source = files.map(f => fs.readFileSync(f, "utf8")).join("\n");
const constants = new Map();
for (const m of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) constants.set(m[1], m[2]);
const detectors = [];
const re = /REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*CORSAIR_VID\s*,\s*([^,\s)]+)/g;
for (const m of source.matchAll(re)) { const pid = constants.get(m[3]) ?? (/^0x/i.test(m[3]) ? m[3] : undefined); if (pid) detectors.push({ name:m[1], handler:m[2], vid:"0x1b1c", pid }); }
if (detectors.length !== 39) throw new Error(`expected 39 detectors, found ${detectors.length}`);
const out = ["// SPDX-License-Identifier: GPL-2.0-or-later", `// Generated from OpenRGB ${commit}.`, "use super::table::Detector;", "pub static DETECTORS: &[Detector] = &[", ...detectors.map(d => `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid} },`), "];", ""].join("\n");
fs.writeFileSync("modules/corsair-peripheral/src/devices_table.rs", out);
console.log(`generated ${detectors.length} Corsair HID detectors`);
