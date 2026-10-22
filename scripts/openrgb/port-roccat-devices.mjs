// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from "node:fs";
import path from "node:path";

const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "RoccatController");
const files = [];
const walk = (dir) => {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(file);
    else if (/\.(cpp|h)$/.test(entry.name)) files.push(file);
  }
};
walk(family);
const source = files.map((file) => fs.readFileSync(file, "utf8")).join("\n");
const constants = new Map();
for (const match of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)) {
  constants.set(match[1], match[2]);
}
const detectors = [];
const pattern = /REGISTER_HID_DETECTOR(?:_[A-Z]+)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*([A-Z0-9_]+)/g;
for (const match of source.matchAll(pattern)) {
  const vid = constants.get(match[3]);
  const pid = constants.get(match[4]);
  if (vid && pid) detectors.push({ name: match[1], handler: match[2], vid, pid });
}
if (detectors.length !== 28) throw new Error(`expected 28 Roccat detectors, found ${detectors.length}`);
const protocol = (handler, name) => {
  if (handler.includes("Vulcan") || handler.includes("Horde")) return "keyboard";
  if (handler.includes("Elo")) return "headset";
  if (handler.includes("Sense")) return "mat";
  if (handler.includes("Kova")) return "kova";
  if (handler.includes("KoneAimo") || handler.includes("Mouse")) return "mouse";
  if (handler.includes("Burst") || handler.includes("KonePro")) return "direct-mouse";
  return name.toLowerCase().includes("air") ? "direct-mouse" : "mouse";
};
const devices = detectors.map((device) => ({ ...device, protocol: protocol(device.handler, device.name) }));
const out = [
  "// SPDX-License-Identifier: GPL-2.0-or-later",
  `// Generated from OpenRGB ${commit}.`,
  "use super::table::{Detector, Device};",
  "pub static DETECTORS: &[Detector] = &[",
  ...devices.map((d) => `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid}, protocol: ${JSON.stringify(d.protocol)} },`),
  "];",
  "pub static DEVICES: &[Device] = &[",
  ...devices.map((d) => `    Device { vid: ${d.vid}, pid: ${d.pid}, protocol: ${JSON.stringify(d.protocol)} },`),
  "];",
  "",
];
fs.writeFileSync("modules/roccat/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} Roccat detectors`);
