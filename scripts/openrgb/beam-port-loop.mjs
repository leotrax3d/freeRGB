#!/usr/bin/env node
// Unattended OpenRGB porting loop for a small model (BEAM via opencode).
// For each backlog family: fresh worktree + branch, one BEAM run with the task
// prompt from docs/promts/beam-openrgb-ports.md, path check, local gate,
// one repair round, then commit + push (+ PR if gh is available) or record BLOCKED.
// State survives restarts in <worktree-root>/beam-ports-state.json.
//
// usage: node scripts/openrgb/beam-port-loop.mjs --openrgb <checkout> [--max N] [--dry-run]
// env:   BEAM_CMD   (default: ollama launch opencode --model beam:cloud -- run --auto)
//        WT_ROOT    (default: ../freergb-wt next to the repo)
//        BUILD_ROOT (default: ../freergb-build)
//        NO_PUSH=1  keep branches local
import { execSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repo = execSync("git rev-parse --show-toplevel").toString().trim();
const args = process.argv.slice(2);
const opt = (n, d) => { const i = args.indexOf(n); return i >= 0 ? args[i + 1] : d; };
const dryRun = args.includes("--dry-run");
const max = Number(opt("--max", "1000"));
const openrgb = opt("--openrgb", process.env.OPENRGB_DIR);
const wtRoot = process.env.WT_ROOT || path.resolve(repo, "..", "freergb-wt");
const buildRoot = process.env.BUILD_ROOT || path.resolve(repo, "..", "freergb-build");
const beamCmd = process.env.BEAM_CMD || "ollama launch opencode --model beam:cloud -- run --auto";
const statePath = path.join(wtRoot, "beam-ports-state.json");
const logPath = path.join(wtRoot, "beam-ports-log.md");
const TIMEOUT_MS = 60 * 60 * 1000;

const sh = (cmd, cwd = repo, quiet = false) => {
  const r = spawnSync(cmd, { cwd, shell: true, encoding: "utf8", maxBuffer: 1 << 28, timeout: TIMEOUT_MS });
  if (!quiet && r.status !== 0) process.stderr.write(r.stdout + r.stderr);
  return { ok: r.status === 0, out: (r.stdout || "") + (r.stderr || "") };
};
const log = (line) => { console.log(line); fs.appendFileSync(logPath, `- ${new Date().toISOString()} ${line}\n`); };

// ---- queue from the backlog -------------------------------------------------
const tierOf = (transport, needs) => {
  if (/USB HID/.test(transport) && /^none/.test(needs)) return 1;
  if (/^A5|^A6/.test(needs)) return 2;
  if (/serial/.test(needs)) return 3;
  if (/SMBus|I2C/.test(needs)) return 4;
  return 5;
};
const exampleFor = (tier, name) => {
  if (tier === 2 || /Keyboard/i.test(name)) return "hyperx-keyboard";
  if (/Mouse(?!mat)/i.test(name)) return "asus-aura-mouse";
  return "tecknet";
};
const slug = (cls) => cls.replace(/Controller$/, "").replace(/HyperX/g, "Hyperx").replace(/ViewSonic/g, "Viewsonic").replace(/SRGBmods/g, "Srgbmods").replace(/NZXT/g, "Nzxt").replace(/([a-z0-9])([A-Z])/g, "$1-$2")
  .replace(/([A-Z]+)([A-Z][a-z])/g, "$1-$2").toLowerCase();

// Classes already (partly) ported by someone: their name appears in a module's PROVENANCE.md.
const ported = fs.readdirSync(path.join(repo, "modules"))
  .map((m) => path.join(repo, "modules", m, "PROVENANCE.md")).filter((f) => fs.existsSync(f))
  .map((f) => fs.readFileSync(f, "utf8")).join("\n");
const baseOf = (cls) => cls.replace(/Controller$/, "");

function queue(state) {
  const rows = fs.readFileSync(path.join(repo, "docs/OPENRGB_BACKLOG.md"), "utf8").split("\n")
    .filter((l) => /^\| [A-Za-z0-9]+Controller /.test(l))
    .map((l) => l.split("|").map((c) => c.trim()))
    .map(([, cls, n, transport, needs]) => ({ cls, n: Number(n), transport, needs }));
  return rows
    .map((r) => ({ ...r, family: slug(r.cls), tier: tierOf(r.transport, r.needs) }))
    .filter((r) => !state[r.family] && !fs.existsSync(path.join(repo, "modules", r.family)))
    .filter((r) => !new RegExp(`Controllers/${baseOf(r.cls)}(Controller)?/`).test(ported))
    .sort((a, b) => a.tier - b.tier || b.n - a.n);
}

// ---- task prompt -------------------------------------------------------------
const doc = fs.readFileSync(path.join(repo, "docs/promts/beam-openrgb-ports.md"), "utf8");
const template = doc.split("## Task prompt")[1].split("```text")[1].split("```")[0];
const commit = /OPENRGB_COMMIT:\s*([0-9a-f]{40})/.exec(fs.readFileSync(path.join(repo, ".github/workflows/modules.yml"), "utf8"))[1];
const promptFor = (r, example) => template
  .replaceAll("<FAMILY>", r.family).replaceAll("<EXAMPLE>", example)
  .replace("(path given by the human, at the commit in .github/workflows/modules.yml, OPENRGB_COMMIT)",
    `(${openrgb}, commit ${commit})`)
  .replace("Controllers/<FAMILY-folder>/", `Controllers/ (the folder that contains ${r.cls}.cpp)`) +
  `\nTier ${r.tier}, backlog tag: ${r.needs}, transport: ${r.transport}.` +
  (r.tier === 4 ? "\nThis family uses SMBus/I2C: port it against the simulated bus API (crates/freergb-module-api/src/bus.rs) with byte-exact tests; do not STOP for step 3 because of SMBus." : "") +
  (r.tier === 3 ? "\nThis family uses serial/network: use crates/freergb-module-api/src/transport.rs; do not STOP for step 3 because of serial." : "");

// ---- checks -----------------------------------------------------------------
const allowed = (f, fam) => f.startsWith(`modules/${fam}/`) || f === `scripts/openrgb/port-${fam}-devices.mjs`
  || f === "Cargo.toml" || f === "Cargo.lock" || f === "docs/OPENRGB_PORTING.md";
function cleanStray(wt, fam) {
  const files = sh("git status --porcelain", wt, true).out.split("\n").filter(Boolean).map((l) => l.slice(3).trim());
  for (const f of files.filter((f) => !allowed(f, fam))) {
    sh(`git checkout -- "${f}"`, wt, true); sh(`git clean -fdq -- "${f}"`, wt, true);
    log(`${fam}: reverted stray change ${f}`);
  }
}
const gate = (wt, fam) => {
  for (const c of ["cargo fmt --all", `cargo clippy -p freergb-module-${fam} --all-targets -- -D warnings`,
    `cargo test -p freergb-module-${fam}`, `cargo clippy -p freergb-module-${fam} --all-targets --locked -- -D warnings`]) {
    const r = sh(c, wt, true);
    if (!r.ok) return { ok: false, cmd: c, out: r.out.slice(-4000) };
  }
  return { ok: true };
};
const runBeam = (wt, text, name) => {
  const f = path.join(wt, ".beam-task.md");
  fs.writeFileSync(f, text);
  const r = sh(`${beamCmd} --title "${name}" -f .beam-task.md "Do the task in the attached file .beam-task.md exactly. Work in this directory."`, wt, true);
  fs.rmSync(f, { force: true });
  return r.out;
};

// ---- main loop ---------------------------------------------------------------
fs.mkdirSync(wtRoot, { recursive: true });
const state = fs.existsSync(statePath) ? JSON.parse(fs.readFileSync(statePath, "utf8")) : {};
const save = () => fs.writeFileSync(statePath, JSON.stringify(state, null, 2));
const q = queue(state);
if (dryRun) { for (const r of q.slice(0, max)) console.log(`tier ${r.tier}  ${r.n.toString().padStart(4)}  ${r.family}  (${r.cls}, ${r.needs})`); process.exit(0); }
if (!openrgb || !fs.existsSync(path.join(openrgb, "Controllers"))) {
  console.error("need --openrgb <OpenRGB checkout at " + commit + ">"); process.exit(2);
}
const hasGh = sh("gh --version", repo, true).ok;

for (const r of q.slice(0, max)) {
  const fam = r.family, branch = `module/${fam}`, wt = path.join(wtRoot, `beam-${fam}`);
  log(`${fam}: start (tier ${r.tier}, ${r.n} detectors)`);
  sh("git fetch -q origin main", repo, true);
  if (!fs.existsSync(wt) && !sh(`git worktree add -q -B ${branch} "${wt}" origin/main`).ok) {
    state[fam] = { status: "error", why: "worktree" }; save(); continue;
  }
  fs.mkdirSync(path.join(wt, ".cargo"), { recursive: true });
  const setup = path.join(repo, "scripts/setup-worktree-build.ps1");
  if (process.platform === "win32" && fs.existsSync(setup)) sh(`powershell -NoProfile -File "${setup}"`, wt, true);
  if (!fs.existsSync(path.join(wt, ".cargo/config.toml")))
    fs.writeFileSync(path.join(wt, ".cargo/config.toml"), `[build]\ntarget-dir = "${path.join(buildRoot, "beam").replaceAll("\\", "/")}"\n`);

  let out = runBeam(wt, promptFor(r, exampleFor(r.tier, r.cls)), fam);
  const blocked = /BLOCKED:\s*(.*)/.exec(out);
  cleanStray(wt, fam);
  let g = blocked ? { ok: false } : gate(wt, fam);
  if (!blocked && !g.ok) {
    log(`${fam}: gate failed (${g.cmd}), repair round`);
    out = runBeam(wt, `Fix ONLY these errors in modules/${fam}/. Same rules as before (only your files, no git push).\nCommand: ${g.cmd}\n\n${g.out}\n\nWhen done write DONE, or BLOCKED: <why>.`, `${fam}-fix`);
    cleanStray(wt, fam);
    g = gate(wt, fam);
  }
  if (blocked || !g.ok) {
    const why = blocked ? blocked[1].trim() : `gate: ${g.cmd}`;
    state[fam] = { status: "blocked", why, cls: r.cls }; save();
    log(`${fam}: BLOCKED ${why}`);
    sh(`git worktree remove --force "${wt}"`, repo, true); sh(`git branch -D ${branch}`, repo, true);
    continue;
  }
  sh(`git add -A -- modules/${fam} scripts/openrgb Cargo.toml Cargo.lock docs/OPENRGB_PORTING.md`, wt, true);
  sh(`git diff --cached --quiet || git commit -qm "modules: port ${fam} (${r.cls}) via BEAM loop"`, wt, true);
  if (!process.env.NO_PUSH) {
    sh(`git push -q -u origin ${branch}`, wt);
    if (hasGh) sh(`gh pr create --head ${branch} --title "modules: port ${fam}" --body "Automated BEAM port of ${r.cls} (${r.n} detectors). Local gate: fmt, clippy -D warnings, crate tests passed."`, wt, true);
  }
  state[fam] = { status: "done", cls: r.cls, branch }; save();
  log(`${fam}: DONE -> ${branch}`);
  sh(`git worktree remove --force "${wt}"`, repo, true);
}
log("queue finished");
