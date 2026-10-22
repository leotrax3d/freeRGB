// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "NZXTHue2Controller");
const files = fs.readdirSync(family).filter((f) => /\.(cpp|h)$/.test(f)).map((f) => path.join(family, f));
const source = files.map((f) => fs.readFileSync(f, "utf8")).join("\n");
const values = new Map();
for (const m of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) values.set(m[1], m[2]);
const detectors = [];
const re = /REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*([A-Z0-9_]+)/g;
for (const m of source.matchAll(re)) { const vid=values.get(m[3]), pid=values.get(m[4]); if (vid && pid) detectors.push({name:m[1],handler:m[2],vid,pid}); }
if (detectors.length !== 21) throw new Error(`expected 21 NZXT detectors, found ${detectors.length}`);
const channels = (handler) => handler.includes("6Channel") ? 6 : handler.includes("Hue2") && !handler.includes("Motherboard") && !handler.includes("Ambient") ? 4 : handler.includes("Ambient") ? 2 : handler.includes("KrakenX3") ? 3 : handler.includes("KrakenElite") ? 2 : handler.includes("RGBController") ? 3 : 2;
const devices = detectors.map((d) => ({...d, channels:channels(d.handler)}));
const out = ["// SPDX-License-Identifier: GPL-2.0-or-later", `// Generated from OpenRGB ${commit}.`, "use super::table::Detector;", "pub static DETECTORS: &[Detector] = &[", ...devices.map((d)=>`    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid}, channels: ${d.channels} },`), "];", ""];
fs.writeFileSync("modules/nzxt-hue2/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} NZXT detectors`);
