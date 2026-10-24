// SPDX-License-Identifier: MIT OR Apache-2.0
// Compare two compatibility JSON files (pinned vs. newer OpenRGB) as Markdown.
// Usage: node scripts/openrgb/compat-drift.mjs <old.json> <new.json>
import { readFileSync } from "node:fs";

const [a, b] = process.argv.slice(2).map((p) => JSON.parse(readFileSync(p, "utf8")));
const key = (d) => `${d.family}|${d.name}|${d.ids}`;
const oldKeys = new Map(a.devices.map((d) => [key(d), d]));
const newKeys = new Map(b.devices.map((d) => [key(d), d]));
const added = [...newKeys.values()].filter((d) => !oldKeys.has(key(d)));
const removed = [...oldKeys.values()].filter((d) => !newKeys.has(key(d)));
const row = (d) => `| ${d.name} | \`${d.family}\` | ${d.transport} | ${d.ids ?? "dynamic"} |`;
console.log(`## OpenRGB drift ${a.openrgb_commit.slice(0, 8)} → ${b.openrgb_commit.slice(0, 8)}\n`);
console.log(`Added: ${added.length}, removed: ${removed.length}.\n`);
for (const [title, list] of [["Added upstream", added], ["Removed upstream", removed]]) {
  if (!list.length) continue;
  console.log(`### ${title}\n\n| Device | Family | Transport | IDs |\n|---|---|---|---|\n${list.map(row).join("\n")}\n`);
}
