// SPDX-License-Identifier: MIT OR Apache-2.0
// Motherboard fan coverage matrix per fan-controller chip (docs/FAN_COVERAGE.md).
// Usage: node scripts/cooling/fan-coverage.mjs <lhm-checkout> <lhm-tag>
// Reads the pinned LibreHardwareMonitor sources (chip enum, driver classes,
// control counts, EC boards) and freeRGB's own tables: chip families with
// simulators (crates/freergb-control/tests/fixtures/chip_families.json), the
// hwmon driver table (crates/freergb-daemon/src/hwmon.rs), the audited
// register-readback list (providers/mainboard/NativeNctEvidence.cs) and
// hardware evidence (scripts/cooling/coverage-evidence.json). Nothing is ever
// promoted to hardware-verified automatically.
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const [checkout, tag] = process.argv.slice(2);
if (!checkout || !/^v\d+\.\d+\.\d+$/.test(tag ?? "")) {
  console.error("usage: fan-coverage.mjs <lhm-checkout> <lhm-tag, e.g. v0.9.6>");
  process.exit(2);
}
const root = process.cwd();
const read = (...p) => readFileSync(join(...p), "utf8");
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
const lpc = join(checkout, "LibreHardwareMonitorLib/Hardware/Motherboard/Lpc");

const pin = JSON.parse(read(root, "packaging/windows/mainboard-packages.json")).packages.lhm.version;
if (`v${pin}` !== tag) {
  console.error(`LHM tag ${tag} does not match the pinned version ${pin}`);
  process.exit(1);
}

// --- LHM: chip enum, display names and driver class per chip.
const chipEnum = strip(read(lpc, "Chip.cs"));
const enumBody = chipEnum.match(/enum Chip[^{]*\{([\s\S]*?)\}/)[1];
const chips = [...enumBody.matchAll(/(\w+)\s*=\s*0x([0-9A-Fa-f]+)/g)].map((m) => ({ chip: m[1], id: m[2].toUpperCase() }));
const names = Object.fromEntries([...chipEnum.matchAll(/case Chip\.(\w+): return "([^"]+)";/g)].map((m) => [m[1], m[2]]));
const lpcIo = strip(read(lpc, "LpcIO.cs"));
const classOf = {};
for (const m of lpcIo.matchAll(/((?:case Chip\.\w+:\s*)+)[\s\S]*?_superIOs\.Add\(new (\w+)\(/g)) {
  for (const c of m[1].matchAll(/Chip\.(\w+)/g)) classOf[c[1]] ??= m[2];
}
// IT87XX is constructed for every ITE chip outside the switch above.
for (const { chip } of chips) if (/^IT8/.test(chip)) classOf[chip] ??= "IT87XX";
// IPMI (server BMCs) is not a motherboard SuperIO and is out of scope.

// Array sizes (`Controls`/`Fans = new float?[N]`) per chip from a driver
// constructor: case labels are tracked by indentation, `default:` covers the
// chips not named at that switch level, ternaries on `chip` are evaluated.
function sizes(text, field, members) {
  const out = {};
  const frames = [];
  const evaluate = (expr, chip) => {
    expr = expr.trim();
    while (/^\(.*\)$/.test(expr) && balanced(expr.slice(1, -1))) expr = expr.slice(1, -1).trim();
    const q = topLevel(expr, "?");
    if (q < 0) return Number(expr);
    const rest = expr.slice(q + 1);
    const c = topLevel(rest, ":");
    const cond = [...expr.slice(0, q).matchAll(/Chip\.(\w+)/g)].map((m) => m[1]);
    return evaluate(cond.includes(chip) ? rest.slice(0, c) : rest.slice(c + 1), chip);
  };
  const assign = (expr, targets) => {
    for (const chip of targets) {
      const n = evaluate(expr, chip);
      if (Number.isInteger(n)) out[chip] = n;
    }
  };
  for (const line of strip(text).split("\n")) {
    if (!line.trim()) continue;
    const indent = line.search(/\S/);
    const label = line.match(/^\s*(?:case Chip\.(\w+)|(default))\s*:/);
    if (label) {
      while (frames.length && frames.at(-1).indent > indent) frames.pop();
      let top = frames.at(-1);
      if (!top || top.indent < indent) frames.push((top = { indent, labels: [], seen: new Set(), collecting: false }));
      if (top.collecting) top.labels = [];
      top.collecting = false;
      const name = label[1] ?? "default";
      top.labels.push(name);
      if (name !== "default") top.seen.add(name);
      continue;
    }
    while (frames.length && indent <= frames.at(-1).indent) frames.pop();
    if (frames.length) frames.at(-1).collecting = true;
    const m = line.match(new RegExp(`\\b${field}\\s*=\\s*new float\\?\\[(.+)\\];`));
    if (!m) continue;
    if (!frames.length) {
      assign(m[1], members);
      continue;
    }
    const resolve = (level) => {
      const frame = frames[level];
      const parent = level > 0 ? resolve(level - 1) : members;
      return frame.labels.flatMap((l) => (l === "default" ? parent.filter((c) => !frame.seen.has(c)) : [l]));
    };
    assign(m[1], resolve(frames.length - 1).filter((c) => members.includes(c)));
  }
  return out;
}
function balanced(s) {
  let d = 0;
  for (const ch of s) {
    d += ch === "(" ? 1 : ch === ")" ? -1 : 0;
    if (d < 0) return false;
  }
  return d === 0;
}
function topLevel(s, needle) {
  let d = 0;
  for (let i = 0; i < s.length; i++) {
    d += s[i] === "(" ? 1 : s[i] === ")" ? -1 : 0;
    if (d === 0 && s[i] === needle) return i;
  }
  return -1;
}
const classFiles = { Nct677X: "Nct677X.cs", IT87XX: "IT87XX.cs", F718XX: "F718XX.cs", W836XX: "W836XX.cs" };
const controls = {};
const fans = {};
for (const [cls, file] of Object.entries(classFiles)) {
  const members = Object.keys(classOf).filter((c) => classOf[c] === cls);
  const text = read(lpc, file);
  Object.assign(controls, sizes(text, "Controls", members));
  Object.assign(fans, sizes(text, "Fans", members));
}

// EC boards (read-only sensors): models listed with fan sensors.
const ecText = strip(read(lpc, "EC/EmbeddedController.cs"));
const ecBoards = [];
for (const m of ecText.matchAll(/new\s*\(\s*(new\s*\[\]\s*\{[^}]*\}|Model\.\w+)\s*,\s*BoardFamily\.(\w+)((?:\s*,\s*ECSensor\.\w+)*)\s*\)/g)) {
  const sensors = [...m[3].matchAll(/ECSensor\.(\w+)/g)].map((s) => s[1]);
  for (const model of m[1].matchAll(/Model\.(\w+)/g)) {
    ecBoards.push({ model: model[1], family: m[2], fan_sensors: sensors.filter((s) => s.startsWith("Fan")).length });
  }
}
ecBoards.sort((a, b) => (a.model < b.model ? -1 : a.model > b.model ? 1 : 0));

// --- freeRGB tables.
const families = JSON.parse(read(root, "crates/freergb-control/tests/fixtures/chip_families.json")).families;
const audited = JSON.parse(read(root, "providers/mainboard/NativeNctEvidence.cs").match(/Audited\s*=\s*(\{[^}]*\})/)[1].replace("{", "[").replace("}", "]"));
const evidence = JSON.parse(read(root, "scripts/cooling/coverage-evidence.json"));
const hwmonText = read(root, "crates/freergb-daemon/src/hwmon.rs");
const drivers = [...hwmonText.matchAll(/HwmonDriver \{ driver: "([^"]+)", family: "([^"]+)", names: &\[([^\]]*)\], control: (true|false), full_speed: (None|Some\((\d+)\)), automatic: &\[([^\]]*)\] \}/g)].map((m) => ({
  driver: m[1],
  family: m[2],
  names: [...m[3].matchAll(/"([^"]+)"/g)].map((n) => n[1]),
  control: m[4] === "true",
  full_speed_enable: m[6] === undefined ? null : Number(m[6]),
  automatic_enable: m[7].split(",").map((s) => s.trim()).filter(Boolean).map(Number),
}));
if (!drivers.length) throw new Error("hwmon DRIVERS table not found");

const errors = [];
const familyOf = {};
for (const f of families.filter((f) => !f.synthetic)) {
  for (const chip of f.chips) {
    if (!classOf[chip]) errors.push(`family ${f.id}: ${chip} is not an LHM ${tag} SuperIO chip`);
    else if (classOf[chip] !== f.lhm_class) errors.push(`family ${f.id}: ${chip} uses ${classOf[chip]}, not ${f.lhm_class}`);
    if (familyOf[chip]) errors.push(`${chip} is in ${familyOf[chip].id} and ${f.id}`);
    familyOf[chip] = f;
  }
}
for (const d of drivers) if (!families.some((f) => f.id === d.family)) errors.push(`hwmon driver ${d.driver}: unknown family ${d.family}`);
for (const e of evidence.entries) {
  if (e.status === "hardware-verified" && !(evidence.hardware_verified_allowed ?? []).includes(e.chip)) {
    errors.push(`${e.chip}: only ${evidence.hardware_verified_allowed} may be marked hardware-verified`);
  }
}

const vendorOf = (name) => (name ?? "").split(" ")[0] || "unknown";
const rows = chips
  .filter(({ chip }) => classOf[chip])
  .map(({ chip, id }) => {
    const family = familyOf[chip];
    const n = controls[chip] ?? 0;
    const tier = n === 0 ? "read-only" : audited.includes(chip) && family?.capability === "verified" ? "verified" : "lhm-supported";
    const ev = evidence.entries.filter((e) => e.chip === chip);
    const verified = ev.find((e) => e.status === "hardware-verified");
    const community = ev.filter((e) => e.status === "community-reported");
    const status = verified ? "hardware-verified" : family ? "simulated" : "unsupported";
    // Linux backend only where the driver lists this chip name (no guessing).
    const hwmon = drivers.find((d) => d.driver === family?.hwmon_driver && d.names.some((n) => chip.toLowerCase().startsWith(n)));
    if (n > 0 && !family) errors.push(`${chip}: ${n} LHM controls but no simulator family`);
    return {
      chip,
      name: names[chip] ?? chip,
      chip_id: `0x${id}`,
      vendor: family?.vendor ?? vendorOf(names[chip]),
      typical_boards: family?.boards ?? [],
      lhm_class: classOf[chip],
      controls: n,
      fans: fans[chip] ?? 0,
      family: family?.id ?? null,
      backends: [`lhm ${tag}`, ...(hwmon ? [`hwmon ${hwmon.driver}${hwmon.control ? "" : " (read-only)"}`] : [])],
      tier,
      unrestricted_unlock: n > 0,
      register_readback: tier === "verified",
      status,
      evidence: verified
        ? verified.link
        : family
          ? "crates/freergb-control/tests/chip_families.rs"
          : null,
      hardware_scope: verified?.scope ?? null,
      community_reports: community.map((e) => ({ board: e.board, bios: e.bios, result: e.result, link: e.link })),
    };
  });
if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}

const count = (key, value) => rows.filter((r) => r[key] === value).length;
const summary = {
  chips: rows.length,
  controllable: rows.filter((r) => r.controls > 0).length,
  tiers: Object.fromEntries(["verified", "lhm-supported", "read-only"].map((t) => [t, count("tier", t)])),
  status: Object.fromEntries(["hardware-verified", "simulated", "unsupported"].map((s) => [s, count("status", s)])),
  ec_boards: ecBoards.length,
};
const json = {
  generated_by: "scripts/cooling/fan-coverage.mjs",
  lhm_version: tag,
  summary,
  families: families.filter((f) => !f.synthetic).map(({ id, name, capability, hwmon_driver, source }) => ({ id, name, capability, hwmon_driver, source })),
  chips: rows,
  hwmon_drivers: drivers,
  ec_boards: ecBoards,
};
writeFileSync(join(root, "docs/fan-coverage.json"), `${JSON.stringify(json, null, 2)}\n`);

const rel = (p) => (/^https?:/.test(p) ? p : p.startsWith("docs/") ? p.slice(5) : `../${p}`);
const md = [];
md.push(`# Motherboard fan coverage`, "");
md.push(`Generated by \`scripts/cooling/fan-coverage.mjs\` from LibreHardwareMonitor ${tag} (the pinned version) and freeRGB's own tables. Do not edit by hand; CI (\`.github/workflows/fan-coverage-drift.yml\`) fails when this file is stale.`, "");
md.push("Support is defined per fan-controller chip, not per board: a board is covered when its SuperIO/EC chip is. Columns:", "");
md.push("* **Tier** — `verified`: audited register readback (Nuvoton separate-mode); `lhm-supported`: reduced duty only after a passed guided channel test, RPM plausibility fault; `read-only`: no usable control. Every controllable chip also offers the explicit per-channel `unrestricted` unlock (DECISIONS.md #14).");
md.push("* **Status** — `hardware-verified`: accepted on real hardware within the stated scope ([COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md)); `simulated`: the chip family's register-level simulator passes the channel test, plausibility, fault-return and unlock tests (`crates/freergb-control/tests/chip_families.rs`); `unsupported`: no simulator family yet.");
md.push("* **Community** — accepted hardware reports (`.github/ISSUE_TEMPLATE/fan-control-report.yml`, imported with `scripts/cooling/import-report.mjs`). A report is evidence for that board only and never promotes a chip to hardware-verified.", "");
md.push(`**Summary:** ${summary.chips} LHM SuperIO chips, ${summary.controllable} with fan control; tiers: ${Object.entries(summary.tiers).map(([k, v]) => `${v} ${k}`).join(", ")}; status: ${Object.entries(summary.status).map(([k, v]) => `${v} ${k}`).join(", ")}; ${summary.ec_boards} EC board models (read-only).`, "");
md.push("## SuperIO chips (Windows: LHM)", "");
md.push("| Chip | Vendor | Typical boards | Controls | Backend | Tier | Status | Evidence | Community |", "|---|---|---|---|---|---|---|---|---|");
for (const r of rows) {
  const ev = r.evidence ? `[${r.status === "hardware-verified" ? "acceptance" : "simulator"}](${rel(r.evidence)})${r.hardware_scope ? ` — ${r.hardware_scope}` : ""}` : "—";
  md.push(`| ${r.name} | ${r.vendor} | ${r.typical_boards.join(", ") || "—"} | ${r.controls} | ${r.backends.join("; ")} | ${r.tier} | ${r.status} | ${ev} | ${r.community_reports.length || "—"} |`);
}
md.push("", "## Linux hwmon drivers", "");
md.push("Linux uses the kernel's drivers through sysfs; freeRGB captures and restores `pwmN_enable` exactly and only writes `1` (manual). Per-driver semantics are simulated in `crates/freergb-daemon/src/hwmon.rs` (`every_driver_restores_its_own_enable_semantics`).", "");
md.push("| Driver | Chip names | Family | Control | Full-speed enable | Automatic enable values |", "|---|---|---|---|---|---|");
for (const d of drivers) md.push(`| \`${d.driver}\` | ${d.names.map((n) => `\`${n}*\``).join(", ")} | \`${d.family}\` | ${d.control ? "yes" : "read-only"} | ${d.full_speed_enable ?? "—"} | ${d.automatic_enable.join(", ") || "—"} |`);
md.push("", "## Simulator families", "");
md.push("| Family | Capability | Quirk source |", "|---|---|---|");
for (const f of json.families) md.push(`| \`${f.id}\` — ${f.name} | ${f.capability} | ${f.source} |`);
md.push("", "## ASUS EC boards (read-only sensors)", "");
md.push("| Model | Board family | Fan sensors |", "|---|---|---|");
for (const b of ecBoards) md.push(`| ${b.model} | ${b.family} | ${b.fan_sensors} |`);
writeFileSync(join(root, "docs/FAN_COVERAGE.md"), `${md.join("\n")}\n`);
console.log(`${rows.length} chips, ${summary.controllable} controllable, ${ecBoards.length} EC boards, ${drivers.length} hwmon drivers`);
if (!existsSync(join(root, "docs/fan-coverage.json"))) process.exit(1);
