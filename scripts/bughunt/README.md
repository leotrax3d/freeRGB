# Bughunt pipeline

Full operating contract: `docs/promts/bughunt-lead.md`. Current owner overrides
and cycle evidence: `docs/BUGHUNT.md`.

1. Fetch `origin` before each cycle and record the current `origin/main` commit.
   Start one branch/worktree per package; check other worktree diffs first.
2. Build the browser demo with Vite `--mode demo --outDir <external-dist>`.
   Keep Playwright/axe dependencies and browser binaries outside the repository.
   Run `explore.mjs` with the arguments documented by its `--help` output.
   Its JSON findings and coverage ledger describe suspects, not confirmed bugs.
3. Audit module detection, catalog verification, install/rollback, startup and
   installer lifecycle. Use fixtures for reproducible root-cause evidence.
4. Launch a FILER using [roles/filer.md](roles/filer.md); findings JSON is its
   only local source material. Publish at most ten new issues per cycle.
5. Launch a different TRIAGE agent using [roles/triage.md](roles/triage.md).
   Replay before labelling confirmed. Keep inconclusive findings explicit.
6. Assign confirmed issues to disjoint FIX worktrees using
   [roles/fix.md](roles/fix.md). Review, gate, integrate, rerun and update counts.

At most two Codex workers run at once; workers never spawn agents. Use
interactive `codex --no-daemon` via `hc run --bg`, low reasoning and the smallest
suitable available model (`gpt-6.1-sol` fallback). Wait on Codex quota instead of
switching providers. Report artifacts with `hc send <lead>`, collect them, then
kill/remove only terminals created by this lead.

## Local run (PowerShell)

```powershell
npm.cmd install --prefix E:/freergb-build/pw-tool playwright axe-core
$env:PLAYWRIGHT_BROWSERS_PATH = 'E:/freergb-build/pw-browsers'
# Install Chromium there only if it is not already available.
npm.cmd exec --prefix E:/freergb-build/pw-tool playwright install chromium
Push-Location apps/desktop
npm.cmd ci
npm.cmd run build:demo -- --outDir E:/freergb-build/bughunt/demo
Pop-Location
node scripts/bughunt/explore.mjs E:/freergb-build/bughunt/demo E:/freergb-build/pw-tool E:/freergb-build/bughunt/cycle-1
```

Use a new run-output directory for each authoritative cycle. Runtime-generated
coverage, screenshots and finding JSON must never be committed. The manifest
sets the locales, viewports, navigation order, input samples and action budget.
The output records exercised, failed and omitted actions separately. Native
file pickers, disabled controls and the language selector have explicit limits;
EN and DE run as separate sessions. A bounded crawl does not establish coverage
of every combination, drag operation or control left behind by navigation.
Only confirmed root causes become fixes; do not file every heuristic as a bug.
Cleanup must use the browser object or exact PIDs created by that worker.
Never kill Chromium processes by name: other workers may be using them.

Each finding has `fingerprint` (`area|rule|selector|text-hash`), `area`, `rule`,
`selector`, `repro_steps`, `expected`, `actual`, `locale`, `viewport`, `commit`,
and local `screenshot` path or null. Domain findings add repository-relative
file:line `evidence` and clearly identify static-only vs reproduced evidence.
Empty sections, clipping and unchanged screens are heuristics requiring triage.

Public issue bodies exclude usernames, local paths, emails, serials, credentials
and personal screenshots. Scan the exact final body immediately before posting.
Keep credentials out of repository files, test logs and reports.

Desktop changes require npm build and test; Rust changes require
`scripts/gate.ps1`. Record partial gates/blockers accurately. Configure Cargo
target/cache paths outside the repo with `scripts/setup-worktree-build.ps1`.
Commit as the project team identity with the harness coauthor trailer. Never
alter release workflows or tags. When editing the checked-out main folder is
forbidden, hand the gated commit to the coordinator for integration; do not
change main's ref behind its worktree.
