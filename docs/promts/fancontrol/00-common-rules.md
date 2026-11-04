# Common rules for every FanControl-parity prompt

Read before starting: `docs/HANDOFF.md`, `docs/STATUS.md`,
`docs/FANCONTROL_PARITY.md`, `docs/COOLING_BROAD_SUPPORT_PLAN.md`,
`docs/COOLING_SAFETY.md`, `docs/DECISIONS.md`.

**Workflow**
- Create one branch from current `main` (`cooling/<topic>`), work only in your
  own git worktree, never switch the shared checkout.
- Before writing code, search for existing implementations of the area
  (`crates/freergb-control`, `crates/freergb-daemon/src/cooling_*`,
  `providers/mainboard`, `apps/desktop/src/features/cooling`). Extend them; never
  build a parallel implementation.
- Open one PR to `main` when done; delete the branch after merge.
- Repo language is English (code, comments, docs, commits).

**Safety (non-negotiable)**
- Pumps are never reduced; unknown channels are never treated as case fans.
- Every write path keeps lease, floor, critical-temperature and fail-safe return
  (100 %, then firmware mode). Unrestricted tier rules stay as decided
  (DECISIONS.md #14).
- No hardware writes in tests or CI. Hardware steps run only with the owner
  present and are listed for prompt 07.
- FanControl is proprietary: use only its public docs as a feature reference. No
  code, resources, binaries or decompilation. LHM (MPL-2.0) stays optional.

**Definition of done**
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`, relevant feature-gated process tests and
  `apps/desktop` checks pass locally; CI green.
- Feature has API, storage, missing-data behavior, tests and UI where stated.
- Update `docs/FANCONTROL_PARITY.md` (acceptance matrix row), `docs/STATUS.md`
  (only check a box with evidence) and `docs/HANDOFF.md` if the repo state changed.
- Final report lists: what is simulated only, what needs owner hardware tests
  (concrete steps for prompt 07), and any open decisions.
