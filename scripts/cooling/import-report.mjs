// SPDX-License-Identifier: MIT OR Apache-2.0
// Turns an accepted "Fan control hardware report" (the cooling_hardware section
// of a diagnostics export) into community evidence for docs/FAN_COVERAGE.md.
// Usage: node scripts/cooling/import-report.mjs <report.json> --issue <url> --result works|partly|fails
//        node scripts/cooling/import-report.mjs --self-test
// Then regenerate the matrix with scripts/cooling/fan-coverage.mjs. Reports are
// evidence for one board; they are recorded as community-reported only.
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const evidencePath = join(root, "scripts/cooling/coverage-evidence.json");
const RESULTS = ["works", "partly", "fails"];

/** Validates a report and returns evidence entries (throws on bad input). */
export function entries(report, { issue, result }, matrix) {
  if (report?.report_format !== "freergb-fan-report/1") throw new Error("not a freergb-fan-report/1 payload");
  if (!/^https:\/\/github\.com\/leotrax3d\/FreeRGB\/issues\/\d+$/.test(issue ?? "")) throw new Error("--issue must be a FreeRGB GitHub issue URL");
  if (!RESULTS.includes(result)) throw new Error(`--result must be one of ${RESULTS.join(", ")}`);
  const text = (v, field) => {
    if (typeof v !== "string" || !v.trim() || v.length > 96 || /[\\/@:]|[\u0000-\u001f]/.test(v)) throw new Error(`${field} is missing or not redacted`);
    return v.trim();
  };
  const board = report.board ?? {};
  const name = `${text(board.manufacturer, "board.manufacturer")} ${text(board.product, "board.product")}`;
  const bios = text(board.bios_version, "board.bios_version");
  const allowed = new Set(["channel", "chip", "backend", "coverage", "tier", "register_readback", "test", "tested", "unlocked"]);
  const byChip = new Map();
  for (const c of report.controls ?? []) {
    for (const key of Object.keys(c)) if (!allowed.has(key)) throw new Error(`unexpected control field ${key}`);
    const short = text(c.chip, "controls[].chip").toUpperCase();
    const row = matrix.chips.find((r) => r.chip === short) ?? (short.length >= 6 ? matrix.chips.find((r) => r.chip.startsWith(short)) : undefined);
    if (!row) throw new Error(`chip ${short} is not in docs/fan-coverage.json`);
    const s = byChip.get(row.chip) ?? { controls: 0, passed: 0, failed: 0, backend: c.backend === "hwmon" ? "hwmon" : "lhm" };
    s.controls += 1;
    if (c.test === "passed") s.passed += 1;
    if (c.test === "failed") s.failed += 1;
    byChip.set(row.chip, s);
  }
  if (!byChip.size) throw new Error("report lists no controls");
  return [...byChip].map(([chip, s]) => ({
    chip,
    board: name,
    bios,
    status: "community-reported",
    result: `${result}; ${s.backend}; ${s.controls} controls, channel test ${s.passed} passed / ${s.failed} failed`,
    link: issue,
  }));
}

function merge(evidence, added) {
  const key = (e) => `${e.chip}|${e.board}|${e.bios}|${e.link}`;
  const seen = new Set(evidence.entries.map(key));
  for (const e of added) if (!seen.has(key(e))) evidence.entries.push(e);
  return evidence;
}

function selfTest() {
  const matrix = { chips: [{ chip: "IT8689E" }, { chip: "NCT6798D" }] };
  const report = {
    report_format: "freergb-fan-report/1",
    board: { manufacturer: "Gigabyte", product: "B650 AORUS ELITE", bios_version: "F30" },
    controls: [
      { channel: 1, chip: "it8689e", backend: "lhm", tier: "lhm_supported", test: "passed" },
      { channel: 2, chip: "it8689e", backend: "lhm", tier: "lhm_supported", test: "failed" },
      { channel: 3, chip: "nct6798", backend: "hwmon", tier: "unknown", test: "none" },
    ],
  };
  const opts = { issue: "https://github.com/leotrax3d/FreeRGB/issues/1", result: "partly" };
  const out = entries(report, opts, matrix);
  const ok = out.length === 2 && out[0].status === "community-reported" && out[0].result.includes("1 passed / 1 failed") && out[1].chip === "NCT6798D";
  const rejects = [
    { ...report, board: { ...report.board, product: "C:\\Users\\me" } },
    { ...report, controls: [{ ...report.controls[0], serial: "x" }] },
    { ...report, report_format: "other" },
  ].every((r) => {
    try {
      entries(r, opts, matrix);
      return false;
    } catch {
      return true;
    }
  });
  const merged = merge({ entries: [...out] }, out);
  if (!ok || !rejects || merged.entries.length !== 2) {
    console.error("import-report self-test failed");
    process.exit(1);
  }
  console.log("import-report self-test passed");
}

const args = process.argv.slice(2);
if (args[0] === "--self-test") {
  selfTest();
} else if (args.length) {
  const opt = (name) => args[args.indexOf(name) + 1];
  const report = JSON.parse(readFileSync(args[0], "utf8"));
  const section = report.cooling_hardware ?? report;
  const matrix = JSON.parse(readFileSync(join(root, "docs/fan-coverage.json"), "utf8"));
  const added = entries(section, { issue: opt("--issue"), result: opt("--result") }, matrix);
  const evidence = merge(JSON.parse(readFileSync(evidencePath, "utf8")), added);
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`${added.length} evidence entries; now run scripts/cooling/fan-coverage.mjs`);
} else {
  console.error("usage: import-report.mjs <report.json> --issue <url> --result works|partly|fails | --self-test");
  process.exit(2);
}
