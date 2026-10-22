// Ports the Razer device and detector tables from an OpenRGB checkout into
// modules/razer-chroma/src/devices_table.rs (GPL-2.0-or-later, see
// modules/razer-chroma/PROVENANCE.md).
//
//   node scripts/openrgb/port-razer-devices.mjs <openrgb-checkout> <commit>
//
// The output is deterministic so a re-run against the pinned commit must
// reproduce the committed file byte for byte (checked in CI).
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [root, commit] = process.argv.slice(2);
if (!root || !/^[0-9a-f]{40}$/.test(commit ?? "")) {
  console.error("usage: port-razer-devices.mjs <openrgb-checkout> <40-hex commit>");
  process.exit(2);
}
const dir = join(root, "Controllers/RazerController");
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/.*$/gm, "");
const header = strip(readFileSync(join(dir, "RazerDevices.h"), "utf8"));
const devices = strip(readFileSync(join(dir, "RazerDevices.cpp"), "utf8"));
const detect = strip(readFileSync(join(dir, "RazerControllerDetect.cpp"), "utf8"));
const controllerHeader = strip(readFileSync(join(dir, "RazerController/RazerController.h"), "utf8"));

const constants = new Map();
for (const m of (header + controllerHeader).matchAll(/#define\s+(RAZER_\w+)\s+(0x[0-9A-Fa-f]+|\d+)/g)) {
  constants.set(m[1], Number(m[2]));
}
for (const m of controllerHeader.matchAll(/(RAZER_(?:LED_ID|MATRIX_TYPE)_\w+)\s*=\s*(0x[0-9A-Fa-f]+|\d+)/g)) {
  constants.set(m[1], Number(m[2]));
}
const value = (token) => {
  token = token.trim();
  if (/^(0x[0-9A-Fa-f]+|\d+)$/.test(token)) return Number(token);
  if (!constants.has(token)) throw new Error(`unknown constant ${token}`);
  return constants.get(token);
};
const names = { ZONE_EN_KEYBOARD: "Keyboard" };
const text = (token) => {
  token = token.trim();
  if (token.startsWith('"')) return JSON.parse(token);
  if (names[token]) return names[token];
  throw new Error(`unknown name ${token}`);
};

const zones = new Map();
for (const m of devices.matchAll(/static\s+const\s+razer_zone\s+(\w+)\s*=\s*\{([^}]*)\}/g)) {
  const [name, type, rows, cols] = m[2].split(",").map((s) => s.trim()).filter(Boolean);
  zones.set(m[1], { name: text(name), type: type.replace("ZONE_TYPE_", ""), rows: Number(rows), cols: Number(cols) });
}

const kinds = {
  DEVICE_TYPE_KEYBOARD: "Keyboard", DEVICE_TYPE_MOUSE: "Mouse", DEVICE_TYPE_MOUSEMAT: "Mousemat",
  DEVICE_TYPE_HEADSET: "Headset", DEVICE_TYPE_HEADSET_STAND: "HeadsetStand", DEVICE_TYPE_KEYPAD: "Keypad",
  DEVICE_TYPE_LEDSTRIP: "LightingController", DEVICE_TYPE_SPEAKER: "Speaker", DEVICE_TYPE_LAPTOP: "Laptop",
  DEVICE_TYPE_GAMEPAD: "Accessory", DEVICE_TYPE_ACCESSORY: "Accessory", DEVICE_TYPE_MICROPHONE: "Accessory",
  DEVICE_TYPE_MOTHERBOARD: "Other", DEVICE_TYPE_GPU: "Other", DEVICE_TYPE_COOLER: "Other", DEVICE_TYPE_DRAM: "Other", DEVICE_TYPE_STORAGE: "Other", DEVICE_TYPE_CASE: "Other", DEVICE_TYPE_VIRTUAL: "Other", DEVICE_TYPE_UNKNOWN: "Other", DEVICE_TYPE_LIGHT: "LightingController",
};
const matrices = { 0: "None", 1: "Standard", 2: "Extended", 3: "Linear", 4: "ExtendedArgb", 5: "Custom" };

const table = [];
for (const m of devices.matchAll(/static\s+const\s+razer_device\s+(\w+)\s*=\s*\{([\s\S]*?)\n\};/g)) {
  const body = m[2];
  const zoneBlock = body.match(/\{([^{}]*)\}/)[1];
  const fields = body.replace(/\{[^{}]*\}/, "ZONES").split(",").map((s) => s.trim()).filter(Boolean);
  const [name, pid, type, matrix, transaction, led, rows, cols, , layout] = fields;
  const zoneRefs = zoneBlock.split(",").map((s) => s.trim()).filter((s) => s && s !== "NULL");
  table.push({
    ident: m[1],
    name: text(name),
    pid: value(pid),
    kind: kinds[type] ?? (() => { throw new Error(`unknown type ${type}`); })(),
    matrix: matrices[value(matrix)],
    transaction: value(transaction),
    led: value(led),
    rows: Number(rows),
    cols: Number(cols),
    zones: zoneRefs.map((z) => {
      const zone = zones.get(z.replace("&", ""));
      if (!zone) throw new Error(`unknown zone ${z}`);
      return zone;
    }),
    keyboardLayout: layout !== "NULL",
  });
}
const listed = new Set(
  [...devices.match(/razer_device_list\[\]\s*=\s*\{([\s\S]*?)\};/)[1].matchAll(/&(\w+)/g)].map((m) => m[1]),
);
const used = table.filter((d) => listed.has(d.ident));

const detectors = [];
for (const m of detect.matchAll(/REGISTER_HID_DETECTOR(_IPU|_PU|_I)?\s*\(([^;]*)\)\s*;/g)) {
  const args = m[2].split(",").map((s) => s.trim());
  const [name, fn, vid, pid, ...rest] = args;
  const variant = m[1] ?? "";
  const d = { name: text(name), function: fn, vid: value(vid), pid: value(pid), interface: null, page: null, usage: null };
  if (variant === "_IPU") [d.interface, d.page, d.usage] = rest.map(value);
  else if (variant === "_PU") [d.page, d.usage] = rest.map(value);
  else if (variant === "_I") [d.interface] = rest.map(value);
  detectors.push(d);
}

const hex = (n, w) => `0x${n.toString(16).toUpperCase().padStart(w, "0")}`;
const opt = (v, w) => (v === null ? "None" : `Some(${w ? hex(v, w) : v})`);
let out = `// SPDX-License-Identifier: GPL-2.0-or-later
//
// GENERATED by scripts/openrgb/port-razer-devices.mjs from OpenRGB
// (https://gitlab.com/CalcProgrammer1/OpenRGB) commit ${commit}:
//   Controllers/RazerController/RazerDevices.cpp, RazerDevices.h,
//   RazerControllerDetect.cpp, RazerController/RazerController.h
// Copyright (C) the OpenRGB contributors (Adam Honse and others), GPL-2.0-or-later.
// Modified: translated from C++ tables into Rust data. Do not edit by hand.
use crate::table::{Detector, Device, Matrix, ZoneDef, ZoneType};
use freergb_module_api::DeviceKind;

pub const UPSTREAM_COMMIT: &str = "${commit}";

pub static DEVICES: &[Device] = &[
`;
for (const d of used) {
  out += `    Device { name: ${JSON.stringify(d.name)}, pid: ${hex(d.pid, 4)}, kind: DeviceKind::${d.kind}, matrix: Matrix::${d.matrix}, transaction_id: ${hex(d.transaction, 2)}, led_id: ${hex(d.led, 2)}, rows: ${d.rows}, cols: ${d.cols}, keyboard_layout: ${d.keyboardLayout}, zones: &[`;
  out += d.zones.map((z) => `ZoneDef { name: ${JSON.stringify(z.name)}, kind: ZoneType::${z.type[0]}${z.type.slice(1).toLowerCase()}, rows: ${z.rows}, cols: ${z.cols} }`).join(", ");
  out += `] },\n`;
}
out += `];

pub static DETECTORS: &[Detector] = &[
`;
for (const d of detectors) {
  out += `    Detector { name: ${JSON.stringify(d.name)}, function: ${JSON.stringify(d.function)}, vid: ${hex(d.vid, 4)}, pid: ${hex(d.pid, 4)}, interface: ${opt(d.interface)}, usage_page: ${opt(d.page, 4)}, usage: ${opt(d.usage, 4)} },\n`;
}
out += "];\n";
writeFileSync(new URL("../../modules/razer-chroma/src/devices_table.rs", import.meta.url), out);
console.log(`${used.length} devices, ${detectors.length} detectors`);
