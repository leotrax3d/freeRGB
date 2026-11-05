# Prompt 10 — Faster CI and a working Package workflow

Read `00-common-rules.md` first. Branch: `ci/faster-builds` from `main`.

## Why
A PR CI run takes ~30 min (Windows is the long pole), which blocks every merge.
The Package workflow (installer build) failed on the last release tags after a
few seconds and its 2026-11-05 manual run on `main` failed too.

## Tasks
1. Measure: list the slowest jobs/steps of recent `CI` and `Package` runs
   (`gh run view --json jobs`) and write the baseline into `docs/TESTING.md`.
2. Speed up without dropping coverage: reliable Rust caching (cache keys per
   workspace/lockfile, shared target for feature-gated test steps), avoid
   rebuilding the workspace for each feature-gated `cargo test` step (build
   once, run many), `cargo nextest` if it helps, skip unaffected heavy jobs on
   docs-only changes (path filters), run desktop/website jobs in parallel.
3. Fix the Package workflow: find why runs fail early (tag runs) and why the
   manual run of 2026-11-05 failed; make `workflow_dispatch` on `main` produce
   a verified installer artifact.
4. Keep branch protection compatible (required check names stable).
5. Document the new timings next to the baseline.

## Done when
Typical PR CI is clearly faster (target: under 15 min), all checks still run
on code changes, Package workflow green on `main`, PR open.
