# Prompt for herdr — simple OpenRGB ports (grunt work, no new module API)

Separate run from `herdr-openrgb-full-coverage.md`. That run builds the module
API (phase A) and ports families that need it. **This run ports only families
that already fit the current module API** — table-driven Direct-mode ports with
an existing recipe. It is grunt work: cheapest models, many small PRs.

You are the **coordinator** (Claude in a herdr-core terminal, `hc` CLI).

## Scope
A family is in scope only if all of these hold:
- HID transport, Direct/per-LED colour writes only (hardware effect modes are
  out of scope here).
- No timers/keepalive, no serial/product-string or DMI reads, no shared keyboard
  layout manager, no multi-interface/HID++ receiver logic, no non-HID transport.
- Packet logic fits in one `lib.rs` + `report.rs` like the existing small
  modules (e.g. `modules/tecknet`, `modules/zalman-z-sync`, `modules/thingm-blink`).

Anything else: do not port it. Add it to `docs/OPENRGB_BACKLOG.md` with the
missing API feature (A1–A7 from the full-coverage prompt) and move on.
Never touch `crates/freergb-module-api` or `crates/freergb-modules` in this run.

## Models
| Role | Command |
|---|---|
| Porter (default) | `codex --no-daemon --dangerously-bypass-approvals-and-sandbox -m gpt-6.1-sol -c model_reasoning_effort=low` |
| Porter (fallback) | `claude --permission-mode bypassPermissions --model haiku` (accept the startup dialog with `hc keys <t> down enter`); `--model sonnet` only if haiku fails twice |
| Triage helper | `ollama launch opencode --model beam:cloud` — list detector rows, classify families as "simple" vs "needs API", draft ledger lines |

At most 4 porters at once. No high effort. Workers do not start agents.
Workers report with `hc send <coordinator id> "<result or blocker>"`.

## Steps
1. Triage: from `docs/openrgb-compatibility.json` take every `not-implemented`
   family; the beam helper drafts a "simple / needs-API" classification by
   reading the OpenRGB controller source at the pinned commit
   (`OPENRGB_COMMIT` in `.github/workflows/modules.yml`). Coordinator spot-checks
   and writes the simple list to `target/simple-ports.md` (largest device count first).
   Coordinate with the full-coverage run: families it already claimed are skipped.
2. Per family (one porter, one branch `ofc/simple-<family>`, own worktree):
   follow `docs/promts/port-openrgb-controller-families.md` exactly — generator
   `scripts/openrgb/port-<family>-devices.mjs`, generated `devices_table.rs`,
   byte-exact packet tests against upstream, `COPYING`, `PROVENANCE.md`,
   workspace member, `docs/OPENRGB_PORTING.md` row with commit hash.
3. Gate locally (target dir `E:/freergb-build/<ws>` via gitignored
   `.cargo/config.toml`): `cargo fmt --all --check`,
   `cargo clippy -p freergb-module-<family> --all-targets --locked -- -D warnings`,
   `cargo test -p freergb-module-<family> --locked`. Coordinator runs the full
   workspace gate before merging. No CI waiting.
4. Merge, regenerate the matrix
   (`node scripts/openrgb/compat-matrix.mjs <openrgb-checkout> <OPENRGB_COMMIT>`),
   kill + remove the worker, next family.

## Rules
No UAC, no hardware writes, no installer, no passwords. GPL-2.0-or-later
provenance kept; never copy GPL-2.0-only files. English in repo. Shared files
(`Cargo.toml`, `Cargo.lock`, ledger, matrix): own lines only, rebase before PR.

## Done
Every in-scope family merged; everything else listed in
`docs/OPENRGB_BACKLOG.md` with its blocker. Report counts before/after to the
user and append them to `docs/HANDOFF.md`.
