# Prompt — port the rest of OpenRGB with a very small model (BEAM)

For the person who ports device families in `modules/`. The model is small, so
every task is one family, the steps are fixed, and stopping is always allowed.
Paste the block **"Task prompt"** into BEAM once per family. Replace `<FAMILY>`.
Everything above it is for the human running BEAM.

## Automatic mode (nobody has to step in)

One command runs the whole backlog unattended:

```sh
git clone https://gitlab.com/CalcProgrammer1/OpenRGB ../openrgb
git -C ../openrgb checkout <OPENRGB_COMMIT from .github/workflows/modules.yml>
node scripts/openrgb/beam-port-loop.mjs --openrgb ../openrgb
```

Per family the script (`scripts/openrgb/beam-port-loop.mjs`):
1. takes the next family from `docs/OPENRGB_BACKLOG.md` (tier order below,
   largest first; skips families that already have a module or provenance),
2. creates branch `module/<family>` in a fresh worktree from `origin/main`
   (shared build cache via `scripts/setup-worktree-build.ps1` on Windows),
3. runs BEAM once with the task prompt below (`BEAM_CMD`, default
   `ollama launch opencode --model beam:cloud -- run --auto`),
4. reverts every change outside the allowed paths,
5. runs fmt, clippy `-D warnings` and the crate tests; on failure gives BEAM
   the error output once to repair,
6. on success commits, pushes the branch and opens a PR (if `gh` is logged in);
   on `BLOCKED` or a second failure drops the branch and records the reason,
7. continues with the next family. State is kept in
   `<WT_ROOT>/beam-ports-state.json`, a log in `beam-ports-log.md`, so the loop
   can be stopped and restarted at any time.

Options: `--dry-run` (show the queue), `--max N`, env `WT_ROOT`, `BUILD_ROOT`,
`NO_PUSH=1`, `BEAM_CMD`. Check that `BEAM_CMD` works once by hand before a
long run. The human only reviews/merges the PRs and, now and then, reads the
BLOCKED list in the state file.

## For the human (manual mode, read once)

* One family per BEAM session. Start a fresh session for the next family.
  Small models lose track in long sessions.
* Pick families from `docs/OPENRGB_BACKLOG.md` in this order:
  1. `USB HID` rows tagged `none? (check)` (~54 detectors) — easiest.
  2. `A5? (check)` keyboards (~31, needs the layout API) and the `A6` row.
  3. `A7 (serial)` rows (~49, serial/network transport API now on `main`).
  4. `A7 (SMBus/I2C)` rows (~920: RAM, GPUs, older boards) **last**. freeRGB has
     no real SMBus/I²C access yet, only the simulated bus. Port them against the
     simulated bus with byte-exact tests; they start working once native bus
     access exists (separate core task, not yours).
  Give BEAM the matching API file name (see table below).
* Before the first session: `git pull`, run `scripts/setup-worktree-build.ps1`
  in your worktree (shared build cache, fast linker).
* After BEAM says `DONE`: look at `git diff --stat` yourself. Only these paths
  may change: `modules/<family>/**`, `scripts/openrgb/port-<family>-devices.mjs`,
  your own line in root `Cargo.toml`, `Cargo.lock`, your own row in
  `docs/OPENRGB_PORTING.md`. Anything else → `git checkout` it.
* Run the check yourself (step 9 below). If it fails twice, drop the branch
  and write the family into `docs/OPENRGB_BACKLOG.md` with the error.
* Never let BEAM run `git push`, merge, or touch `crates/`.

| Backlog tag | Needed API | File to give BEAM to read |
|---|---|---|
| A1 timers/keepalive | host tick | `crates/freergb-module-api/src/timing.rs` (or grep `keepalive`) |
| A2 serial/product string | identity reads | grep `identity` in `crates/freergb-module-api/src` |
| A3 board vendor (DMI) | platform identity | grep `platform` in `crates/freergb-module-api/src` |
| A4 hardware effect modes | modes | grep `HardwareMode` in `crates/freergb-module-api/src` |
| A5 keyboard layouts | layouts | grep `layout` in `crates/freergb-module-api/src` |
| A6 multi-interface / HID++ | grouped HID | `crates/freergb-module-api/src/transport/hid.rs` |
| A7 serial/network/SDK | transports | `crates/freergb-module-api/src/transport.rs` |

Pick a model-easy **example module** for BEAM to copy (small, same kind):
`modules/tecknet` (simplest), `modules/zalman-z-sync`, `modules/thingm-blink`
(Direct mode); `modules/asus-aura-mouse` (mouse); `modules/hyperx-keyboard`
(keyboard).

---

## Task prompt (paste this, replace `<FAMILY>` and `<EXAMPLE>`)

```text
You port ONE OpenRGB device family to freeRGB. Family: <FAMILY>.
Example module to copy: modules/<EXAMPLE>.

Rules. Follow them exactly.
- Only create or change files in: modules/<FAMILY>/, scripts/openrgb/port-<FAMILY>-devices.mjs,
  one new line in the root Cargo.toml [workspace] members, one new row in docs/OPENRGB_PORTING.md.
- Do not change anything in crates/. Do not change other modules.
- Do not run git push, git merge, or anything with admin rights. Do not open real devices.
- Write code, comments and docs in English.
- If a step does not work after 2 tries, STOP and write: BLOCKED: <step number> <one line why>.
- Do one step at a time. After each step write: STEP <n> OK.

Step 1. Read these files and nothing else yet:
  - modules/<EXAMPLE>/Cargo.toml, module-info.json, source-workspace.toml, PROVENANCE.md
  - modules/<EXAMPLE>/src/lib.rs, main.rs, table.rs, report.rs, tests.rs (those that exist)
  Write one short line per file: what it does.

Step 2. Find the OpenRGB source of <FAMILY>. It is in the OpenRGB checkout
  (path given by the human, at the commit in .github/workflows/modules.yml, OPENRGB_COMMIT),
  folder Controllers/<FAMILY-folder>/. Read the *Controller.cpp, *Controller.h and the
  *Detect* entries for this family. Check the license header: it must say
  "GPL-2.0-or-later" or "or (at your option) any later version". If it says GPL-2.0-only,
  STOP: BLOCKED: 2 license.

Step 3. Write down, as a short list:
  - every USB vendor id and product id, with the device name
  - the HID interface / usage page / usage if the detector checks them
  - the exact bytes of one "set colors" packet: length, report id, each header byte,
    where the colors go, color order (RGB/GRB/...), checksum if any
  - number of LEDs or zones per device
  If the device needs timers, serial number, several interfaces, or is not USB HID,
  STOP: BLOCKED: 3 needs <A1..A7>.

Step 4. Create scripts/openrgb/port-<FAMILY>-devices.mjs by copying the generator of the
  example (scripts/openrgb/port-<EXAMPLE>-devices.mjs, if it exists) and changing it to
  read the device ids of <FAMILY>. Run it: node scripts/openrgb/port-<FAMILY>-devices.mjs
  It must write modules/<FAMILY>/src/devices_table.rs. If the example has no generator,
  write devices_table.rs by hand from your list in step 3.

Step 5. Create modules/<FAMILY>/ by copying modules/<EXAMPLE>/ and renaming every
  "<EXAMPLE>" to "<FAMILY>" (package name freergb-module-<FAMILY>).
  Change only:
  - src/report.rs: build the packet exactly as in step 3.
  - src/table.rs / src/lib.rs: use devices_table.rs and the LED count.
  - module-info.json: id, name, device ids (same format as the example).
  Keep the structure of the example. Do not invent new features.

Step 6. Add tests in src/tests.rs (copy the test style of the example):
  - one test per packet type that checks every byte against the bytes from step 3
  - one test for the color order
  - one test that the device table contains every id from step 3

Step 7. Copy COPYING from the example. Write PROVENANCE.md like the example:
  OpenRGB commit, the exact source file paths, license line.
  Then run `python scripts/fill-module-provenance.py --openrgb <checkout> --module <FAMILY>`:
  it appends the generated attribution block (upstream paths, URLs at the commit, SPDX and
  copyright/author lines from the file headers). Name every upstream file you used in
  backticks (`File.cpp`) in your hand-written record so the script can resolve it, list the
  changes you made, and do not edit the generated block by hand.

Step 8. Add "modules/<FAMILY>" to the [workspace] members in the root Cargo.toml
  (keep alphabetical order, change nothing else). Add one row to docs/OPENRGB_PORTING.md
  in the "Port status ledger" table, same format as the rows around it.

Step 9. Run, in this order, and fix only errors in your own files:
  cargo fmt --all
  cargo clippy -p freergb-module-<FAMILY> --all-targets --locked -- -D warnings
  cargo test -p freergb-module-<FAMILY> --locked
  If Cargo.lock must change, run the same commands without --locked once, then again with --locked.

Step 10. git add only your files, then
  git commit -m "modules: port <FAMILY> Direct mode"
  Write: DONE <FAMILY> <number of device ids> <number of tests>.
```

---

## After each family (human)
1. Check the diff paths (see above) and rerun step 9.
2. Push the branch `module/<family>` and open a PR, or merge locally if you
   are allowed to.
3. Regenerate the coverage matrix:
   `node scripts/openrgb/compat-matrix.mjs <openrgb-checkout> <OPENRGB_COMMIT>`.
4. Next family in a new BEAM session.

## What BEAM should not do
Big refactors, new API, several families at once, guessing packet bytes.
A `BLOCKED` answer is a good result: write it into `docs/OPENRGB_BACKLOG.md`.
