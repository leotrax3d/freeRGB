# FIX

No subagents. Work on one confirmed issue in branch `fix/bughunt-<issue>` and
its assigned external worktree. Inspect branches/worktrees/diffs first; skip
files owned by running work and tell the lead. Read the issue/replay evidence.
Make a minimal fix within the existing design plus a meaningful regression.
UI text uses i18n keys in both English and German; list new keys for the lead.
Real testbench operations follow the owner authorization in `docs/BUGHUNT.md`.
Run worktree build setup with external target/cache paths before Cargo commands.

Gate desktop with npm build/test, Rust with `scripts/gate.ps1`. Build output
must stay in the external build directory. Record exact results and blockers.
Commit as `freergbteam@gmail.com` with harness coauthor trailer. Do not merge,
push, close issues or alter release workflows/tags: the lead reviews/integrates.
Report `hc send <lead> "issue, result; report path"` with commit and test evidence.
