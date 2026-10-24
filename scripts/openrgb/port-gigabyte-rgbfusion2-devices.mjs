// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const d = path.join(root, "Controllers", "GigabyteRGBFusion2USBController");
const files = [];
const walk = x => { for (const e of fs.readdirSync(x, { withFileTypes: true })) { const p = path.join(x, e.name); if (e.isDirectory()) walk(p); else if (/\.(cpp|h)$/.test(e.name)) files.push(p); } };
walk(d);
const s = files.map(x => fs.readFileSync(x, "utf8")).join("\n");
const v = new Map();
for (const m of s.matchAll(/^\s*#define\s+([A-Za-z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) v.set(m[1], m[2]);
const rows = [];
for (const m of s.matchAll(/REGISTER_HID_DETECTOR[^\(]*\(\s*[^,]+,\s*[^,]+,\s*([A-Za-z0-9_]+)\s*,\s*(0x[0-9A-Fa-f]+)/g)) if (v.has(m[1])) rows.push({ name: "Gigabyte RGB Fusion 2 USB", vid: v.get(m[1]), pid: m[2] });
if (rows.length !== 5) throw new Error(`expected 5 detectors, found ${rows.length}`);
const out = ["// SPDX-License-Identifier: GPL-2.0-or-later", `// Generated from OpenRGB ${commit}.`, "use super::table::{Detector,Device};", "pub static DETECTORS:&[Detector]= &[", ...rows.map(d => `Detector{name:${JSON.stringify(d.name)},vid:${d.vid},pid:${d.pid}},`), "];", "pub static DEVICES:&[Device]= &[", ...rows.map(d => `Device{vid:${d.vid},pid:${d.pid}},`), "];", ""];
fs.mkdirSync("modules/gigabyte-rgbfusion2/src", { recursive: true });
fs.writeFileSync("modules/gigabyte-rgbfusion2/src/devices_table.rs", out.join("\n"));
console.log(`generated ${rows.length} Gigabyte detectors`);
