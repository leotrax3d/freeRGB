# Repository cleanup report

Scan of tracked files (`git ls-files`) before the public release, 2026-11-08.

## (a) Secrets

None found. Searched for private keys, cloud/GitHub/API tokens, JWTs,
`service_role`/secret keys, password literals and `.env`/key files.
`docs/SUPABASE_SETUP.md` only describes where to find the *publishable* key.
The project contact address `freergbteam@gmail.com` is intentionally public.

## (b) Junk and build artefacts

No tracked logs, `dist/`, `node_modules/`, `target/`, agent files
(`opencode.json`, `.s18/`, `.beam-task.md`) or oversized binaries without
purpose. `.gitignore` now also covers `*.log`, `.s18/`, `.beam-task.md`,
`opencode.json` and `.env*`. Largest tracked files are legal/SBOM data and
brand images.

## (c) Personal data in useful docs (redacted)

- `docs/TESTING.md`, `docs/COOLING_ACCEPTANCE.md`: local `E:/freergb-build/...`
  paths replaced by `<build-dir>`.
- `scripts/setup-worktree-build.ps1`: default cache/target directory now under
  `%LOCALAPPDATA%\freergb-build`.
- `scripts/openrgb/port-*.mjs`: hard-coded user temp path removed; the OpenRGB
  checkout must be passed as argument or `OPENRGB_ROOT`.
- `docs/EFFECT_COLLECTION.md`, `docs/MAINBOARD_SMBUS_DRIVER_DECISION.md`:
  user path and first name removed.

**Fixed:** the 25 collection packages under `effects/development/*.wasm` embedded
the build machine path in debug strings. `scripts/build-effects.py` now passes
`--remap-path-prefix` for the checkout, the Rust sysroot and the home directory;
the packages and `effects/development/catalog.json` hashes were rebuilt. The
`effect-host` tests pass.

## (d) Internal agent/coordination docs (kept; owner decision)

Not deleted because agents still use them. Personal paths/names were redacted
(`docs/promts/archive/*`, `docs/promts/herdr-*`); content is otherwise unchanged.

- `docs/promts/` (incl. `archive/`, `fancontrol/`) — agent prompts, partly German
- `docs/HANDOFF.md`, `docs/archive/HANDOFF_2026-10-19.md`, `docs/MARKETPLACE_HANDOFF.md`
- `docs/OPEN_WORK.md`, `docs/UI_GAPS.md`

Suggestion: move these to a `docs/internal/` folder or a private repo after the
release work finishes.

## Not touched (owned by running branches)

`docs/MAINBOARD_RGB.md` and `docs/promts/mainboard-header-rgb-s22-report.md`
(task/mbheaders), `docs/promts/fancontrol/07-hardware-acceptance.md`
(task/legal-*) were not scanned for edits; re-run the scan after those merge.

## History

No history rewrite was performed. Older commits still contain the paths above.
