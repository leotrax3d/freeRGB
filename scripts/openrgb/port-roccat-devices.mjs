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
  if (handler.includes("Horde")) return "horde";
  if (handler.includes("Vulcan")) return "keyboard";
  if (handler.includes("Elo")) return "headset";
  if (handler.includes("Sense")) return "mat";
  if (handler.includes("Kova")) return "kova";
  if (handler.includes("KoneProAir")) return "kone-pro-air";
  if (handler.includes("KoneXP")) return "kone-xp";
  if (handler.includes("KoneAimo") || handler.includes("Mouse")) return "mouse";
  if (handler.includes("BurstProAir")) return "burst-pro-air";
  if (handler.includes("Burst") || handler.includes("KonePro")) return "direct-mouse";
  return name.toLowerCase().includes("air") ? "direct-mouse" : "mouse";
};
const metadata = (name, handler) => {
  const p = protocol(handler, name);
  if (p === "horde") return { p, leds: 6, packet: 24, column: 1, version: 1 };
  if (p === "headset") return { p, leds: 1, packet: 16, column: 1, version: 1 };
  if (p === "mat") return { p, leds: 2, packet: 9, column: 1, version: 1 };
  if (p === "kova") return { p, leds: 2, packet: 46, column: 1, version: 1 };
  if (p === "mouse") return { p, leds: 11, packet: 46, column: 1, version: 1 };
  if (name.includes("Burst Pro Air")) return { p, leds: 4, packet: 30, column: 1, version: 2 };
  if (name.includes("Burst Core")) return { p, leds: 1, packet: 11, column: 1, version: 1 };
  if (name.includes("Burst Pro")) return { p, leds: 2, packet: 11, column: 1, version: 1 };
  if (name.includes("Kone Pro Air")) return { p, leds: 2, packet: 16, column: 1, version: 2 };
  if (name.includes("Kone Pro")) return { p, leds: 2, packet: 11, column: 1, version: 1 };
  if (name.includes("Kone XP")) return { p, leds: 19, packet: 122, column: 1, version: 3 };
  if (name.includes("Horde")) return { p, leds: 6, packet: 24, column: 1, version: 1 };
  if (name.includes("Magma")) return { p, leds: 105, packet: 64, column: 5, version: 2 };
  if (name.includes("Vulcan II Max")) return { p, leds: 148, packet: 567, column: 1, version: 2 };
  if (name.includes("Vulcan II Mini")) return { p, leds: 69, packet: 345, column: 1, version: 2 };
  if (name.includes("Vulcan II TKL")) return { p, leds: 85, packet: 320, column: 1, version: 2 };
  if (name.includes("Vulcan II")) return { p, leds: 105, packet: 396, column: 1, version: 2 };
  if (name.includes("Vulcan TKL")) return { p, leds: 85, packet: 436, column: 12, version: 1 };
  if (name.includes("Vulcan")) return { p, leds: 105, packet: 436, column: 12, version: 1 };
  return { p, leds: 105, packet: 436, column: 12, version: 1 };
};
const devices = detectors.map((device) => ({ ...device, ...metadata(device.name, device.handler) }));
const out = [
  "// SPDX-License-Identifier: GPL-2.0-or-later",
  `// Generated from OpenRGB ${commit}.`,
  "use super::table::{Detector, Device};",
  "pub static DETECTORS: &[Detector] = &[",
  ...devices.map((d) => `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: ${d.vid}, pid: ${d.pid}, protocol: ${JSON.stringify(d.p)} },`),
  "];",
  "pub static DEVICES: &[Device] = &[",
  ...devices.map((d) => `    Device { vid: ${d.vid}, pid: ${d.pid}, protocol: ${JSON.stringify(d.p)}, leds: ${d.leds}, packet: ${d.packet}, column: ${d.column}, version: ${d.version} },`),
  "];",
  "",
];
fs.writeFileSync("modules/roccat/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} Roccat detectors`);
