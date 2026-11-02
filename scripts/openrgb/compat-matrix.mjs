// SPDX-License-Identifier: MIT OR Apache-2.0
// Per-device OpenRGB compatibility matrix covering every detector transport.
// Usage: node scripts/openrgb/compat-matrix.mjs <openrgb-checkout> <40-hex-commit>
// Writes docs/OPENRGB_COMPATIBILITY.md and docs/openrgb-compatibility.json.
// Manual status (hardware verification, native drivers, blockers) lives in
// scripts/openrgb/compat-overrides.json; nothing is ever promoted automatically.
import { readdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, basename } from "node:path";

const [checkout, commit] = process.argv.slice(2);
if (!checkout || !/^[0-9a-f]{40}$/.test(commit ?? "")) {
  console.error("usage: compat-matrix.mjs <openrgb-checkout> <40-hex-commit>");
  process.exit(2);
}
const root = process.cwd();
const walk = (dir) => readdirSync(dir, { withFileTypes: true }).sort((x, y) => (x.name < y.name ? -1 : x.name > y.name ? 1 : 0)).flatMap((e) => {
  const p = join(dir, e.name);
  return e.isDirectory() ? walk(p) : [p];
});
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
const splitArgs = (t) => t.split(/,(?![^()]*\))/).map((s) => s.trim());

// Numeric constants: per family first, then global.
const defineRe = /#define\s+([A-Za-z_]\w*)\s+\(?\s*(0x[0-9A-Fa-f]+|\d+)\s*\)?/g;
const constRe = /\b(?:static\s+)?(?:const(?:expr)?\s+)?(?:unsigned\s+short|uint16_t|unsigned\s+int|int)\s+([A-Za-z_]\w*)\s*=\s*(0x[0-9A-Fa-f]+|\d+)\s*;/g;
const collect = (text, into) => {
  for (const re of [defineRe, constRe]) for (const m of text.matchAll(re)) if (!(m[1] in into)) into[m[1]] = Number(m[2]);
};
const globalConsts = {};
const allSources = walk(checkout).filter((p) => /\.(h|hpp|cpp|c)$/i.test(p) && !p.includes(`${"dependencies"}`));
for (const p of allSources) collect(strip(readFileSync(p, "utf8")), globalConsts);
const hex = (v) => (v === undefined ? null : v.toString(16).padStart(4, "0"));

const kinds = {
  HID: { re: /REGISTER_(?:DYNAMIC_)?HID_(?:WRAPPED_)?DETECTOR(?:_[A-Z]+)*\s*\(/, ids: [2, 3], transport: "USB HID" },
  WOOTING: { re: /REGISTER_WOOTING_DETECTOR\s*\(/, ids: [1, 2], transport: "USB HID" },
  SERIAL: { re: /REGISTER_USB_SERIAL_DETECTOR\s*\(/, ids: [2, 3], transport: "USB serial" },
  PCI: { re: /REGISTER_I2C_PCI_DETECTOR\s*\(/, ids: [2, 3, 4, 5], transport: "I2C/PCI (GPU)" },
  DRAM: { re: /REGISTER_I2C_DRAM_DETECTOR\s*\(/, ids: [], transport: "SMBus DRAM" },
  I2C: { re: /REGISTER_I2C_DETECTOR\s*\(/, ids: [], transport: "SMBus/I2C" },
  OTHER: { re: /REGISTER_(?:DYNAMIC_)?DETECTOR\s*\(/, ids: [], transport: "other (network/serial/OS)" },
};
const registerRe = /\b(REGISTER_[A-Z0-9_]*DETECTOR[A-Z0-9_]*)\s*\(([\s\S]*?)\)\s*;/g;

// freeRGB side: module device tables and the built-in registry.
const modules = {};
for (const dir of readdirSync(join(root, "modules"))) {
  const src = join(root, "modules", dir, "src");
  if (!existsSync(src)) continue;
  const text = walk(src).filter((p) => p.endsWith(".rs")).map((p) => readFileSync(p, "utf8")).join("\n");
  const consts = {};
  for (const m of text.matchAll(/const\s+(\w+)\s*:\s*u16\s*=\s*(0x[0-9A-Fa-f]+)/g)) consts[m[1]] = Number(m[2]);
  const ids = new Set();
  for (const m of text.matchAll(/vid:\s*(0x[0-9A-Fa-f]+|\w+)\s*,[\s\S]{0,120}?pid:\s*(0x[0-9A-Fa-f]+|\w+)/g)) {
    const v = m[1].startsWith("0x") ? Number(m[1]) : consts[m[1]];
    const p = m[2].startsWith("0x") ? Number(m[2]) : consts[m[2]];
    if (v !== undefined && p !== undefined) ids.add(`${hex(v)}:${hex(p)}`);
  }
  if (consts.VID !== undefined) for (const m of text.matchAll(/\b(?:PID|pid)\w*\s*(?::\s*u16\s*)?=\s*(0x[0-9A-Fa-f]+)/g)) ids.add(`${hex(consts.VID)}:${hex(Number(m[1]))}`);
  modules[dir] = ids;
}
const overrides = JSON.parse(readFileSync(join(root, "scripts/openrgb/compat-overrides.json"), "utf8"));

const devices = [];
const controllers = join(checkout, "Controllers");
for (const fam of readdirSync(controllers, { withFileTypes: true }).filter((e) => e.isDirectory())) {
  const files = walk(join(controllers, fam.name)).filter((p) => /\.(cpp|h|hpp|c|cc)$/i.test(p));
  const texts = files.map((p) => strip(readFileSync(p, "utf8")));
  const local = {};
  texts.forEach((t) => collect(t, local));
  const value = (expr) => {
    const e = expr.replace(/[()\s]/g, "");
    if (/^(0x[0-9A-Fa-f]+|\d+)$/.test(e)) return Number(e);
    return local[e] ?? globalConsts[e];
  };
  for (const text of texts) for (const m of text.matchAll(registerRe)) {
    const [macro, body] = [m[1], m[2]];
    if (/DISABLED|BUS_DETECTOR/.test(macro)) continue;
    const kind = Object.entries(kinds).find(([, k]) => k.re.test(`${macro}(`));
    if (!kind) continue;
    const args = splitArgs(body);
    // Skip macro definitions that wrap REGISTER_* themselves.
    const line = text.slice(Math.max(0, m.index - 200), m.index).split("\n").pop();
    if (/##|^name$/.test(args[0]) || /#define/.test(line)) continue;
    const name = args[0].replace(/^"|"$/g, "").replace(/"\s*"/g, "") || args[0];
    const nums = kind[1].ids.map((i) => (args[i] === undefined ? undefined : value(args[i])));
    const any = (expr) => /ANY/.test(expr ?? "");
    let ids = null;
    if (kind[0] === "PCI") ids = nums.map(hex).join(":");
    else if (kind[1].ids.length) ids = `${hex(nums[0]) ?? "?"}:${any(args[kind[1].ids[1]]) ? "*" : hex(nums[1]) ?? "?"}`;
    devices.push({ name: /^[A-Z_]+$/.test(name) ? (globalConsts[name] ?? name) : name, family: fam.name, transport: kind[1].transport, macro, ids });
  }
}

// Deduplicate identical name/family/ids (multiple interfaces of one device).
const seen = new Map();
for (const d of devices) {
  const key = `${d.family}|${d.name}|${d.ids}`;
  if (!seen.has(key)) seen.set(key, d);
}
const rows = [...seen.values()].sort((a, b) => a.family.localeCompare(b.family) || a.name.localeCompare(b.name));

const familyOverride = (fam) => overrides.families?.[fam];
for (const d of rows) {
  const ov = overrides.devices?.[d.ids] ?? null;
  const fo = familyOverride(d.family);
  const mod = Object.entries(modules).find(([, ids]) => d.ids && ids.has(d.ids))?.[0];
  if (ov) Object.assign(d, { implementation: ov.implementation, status: ov.status, functions: ov.functions ?? "", evidence: ov.evidence ?? "" });
  else if (mod) Object.assign(d, { implementation: `module:${mod}`, status: "implemented", functions: fo?.functions ?? "direct colour", evidence: "simulated tests" });
  else if (fo?.status) Object.assign(d, { implementation: fo.implementation ?? "", status: fo.status, functions: fo.functions ?? "", evidence: fo.evidence ?? "" });
  else Object.assign(d, { implementation: "", status: "not-implemented", functions: "", evidence: "" });
}

const byStatus = (list) => list.reduce((acc, d) => ((acc[d.status.split(":")[0]] = (acc[d.status.split(":")[0]] ?? 0) + 1), acc), {});
const transports = [...new Set(rows.map((d) => d.transport))].sort();
const statusCols = ["hardware-verified", "implemented", "blocked", "not-implemented"];
const summary = transports.map((t) => {
  const s = byStatus(rows.filter((d) => d.transport === t));
  return `| ${t} | ${rows.filter((d) => d.transport === t).length} | ${statusCols.map((c) => s[c] ?? 0).join(" | ")} |`;
}).join("\n");
const totals = byStatus(rows);
const families = [...new Set(rows.map((d) => d.family))];
const famTable = families.map((f) => {
  const list = rows.filter((d) => d.family === f);
  const s = byStatus(list);
  const impl = [...new Set(list.map((d) => d.implementation).filter(Boolean))].join(", ") || "—";
  return `| \`${f}\` | ${[...new Set(list.map((d) => d.transport))].join(", ")} | ${list.length} | ${statusCols.map((c) => s[c] ?? 0).join(" | ")} | ${impl} |`;
}).join("\n");
const esc = (s) => String(s ?? "").replace(/\|/g, "\\|");
const devTable = rows.map((d) => `| ${esc(d.name)} | \`${d.family}\` | ${d.transport} | ${d.ids ?? "dynamic"} | ${esc(d.implementation) || "—"} | ${esc(d.functions) || "—"} | ${d.status} |`).join("\n");

writeFileSync(join(root, "docs/openrgb-compatibility.json"), JSON.stringify({ openrgb_commit: commit, devices: rows }, null, 1) + "\n");
writeFileSync(join(root, "docs/OPENRGB_COMPATIBILITY.md"), `# OpenRGB compatibility matrix

GENERATED by \`scripts/openrgb/compat-matrix.mjs\` from OpenRGB commit \`${commit}\`; do not edit by hand.
Manual status lives in \`scripts/openrgb/compat-overrides.json\`. Goal: 100 % of OpenRGB devices.

Status meanings: **hardware-verified** = user-confirmed on real hardware (see DEVICES.md);
**implemented** = code with simulated/unit tests only; **blocked:<reason>** = known blocker;
**not-implemented** = nothing yet. One row per unique detector (name, family, IDs); HID IDs are
\`vid:pid\`, GPU IDs \`ven:dev:subven:subdev\`. Detection-function-only families (SMBus, network)
have no static IDs.

Total: **${rows.length}** device entries in **${families.length}** families —
${statusCols.map((c) => `${c}: ${totals[c] ?? 0}`).join(", ")}.

## By transport

| Transport | Entries | Verified | Implemented | Blocked | Not implemented |
|---|---:|---:|---:|---:|---:|
${summary}

## By family

| Family | Transport | Entries | Verified | Implemented | Blocked | Not implemented | freeRGB implementation |
|---|---|---:|---:|---:|---:|---:|---|
${famTable}

## All devices

| Device | Family | Transport | IDs | freeRGB | Functions | Status |
|---|---|---|---|---|---|---|
${devTable}
`);
console.log(`${rows.length} entries, ${families.length} families`, totals);
