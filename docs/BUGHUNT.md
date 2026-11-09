# Bughunt cycles

## Current owner instructions (2026-11-09)

The owner explicitly designated this computer as a testbench and authorized
real application interactions, device/fan control, installer lifecycle and UAC
tests. This overrides the original prompt's simulation-only restriction.
Keep existing product safety checks intact. No credentials belong in this
repository, worker prompts, test logs or public reports. Fan tests use the app's
safety flow, confirmed channels and verified stop. Installer tests must use
artifacts from the external build tree and restore the machine afterward.
The separate Windows installation on the build drive is out of scope.
Work remains confined to worktrees;
build output and browser tooling remain in the designated external build tree.
Public GitHub bug reporting is explicitly requested.

All agents must use the newest GitHub source. Fetch before each cycle/final
gate and record the exact base commit; update work before gating if it advances.
Installed binaries must be identified separately from source under test.

## Cycle 1 (in progress)

Fetched GitHub `origin/main`: `0b766e38228125c4b9927faeb88c85814e83cf5d`.
Both initial worktrees started at that commit. No conflicting active branches
were present when work began.

Priority owner report: after installation the modern GUI does not automatically
open the setup wizard; reopening through Settings works. Independent
fresh-state reproduction is pending. Starting the existing installed app opened Marketplace
without a wizard, but that is **not** clean-install proof because prior per-user
state may exist. Read-only inspection showed `completed=true`, safety version 3.
Native input initially hit capture/input interference; after retrying, the lead
clicked Settings, Run setup wizard, Welcome, the already-accepted Safety step,
Devices, skip, Fans, skip, Summary and Open freeRGB. The wizard opened and
returned to Lighting. Completion was restored to true. No channel identification,
duty change, installer or UAC operation was performed. Device setup reported
the catalog unavailable; the fans inventory was visible.

The installed desktop and daemon identify as `0.1.0-beta.5`; their exact source
SHA is unknown. The installed Lighting view displayed raw `lighting.hint.*`
keys. Coordinator evidence identifies the existing main fix as `d75b8cc`;
independent latest-source replay must precede closing the report.

Domain audit: `cargo test -p freergb-modules --lib --locked`: 27 passed.
The Linux package correctly rewrites the source service path to `/usr/bin`.
An initial dependency-version suspicion was discarded because no version
contract or failing regression supported it. One static installer finding
remains: the custom Inno installer does not check/provision WebView2.

Public issues (all bodies privacy-scanned by a separate FILER):

- [#1 WebView2 prerequisite](https://github.com/leotrax3d/freeRGB/issues/1):
  static evidence; clean-machine replay pending.
- [#2 Wizard after installation](https://github.com/leotrax3d/freeRGB/issues/2):
  owner-reported, unconfirmed; manual opening works.
- [#3 Raw lighting hints](https://github.com/leotrax3d/freeRGB/issues/3):
  independently replayed EN/DE; fixed in main `d75b8cc`, closed for next release.
- [#4 Demo fan pause](https://github.com/leotrax3d/freeRGB/issues/4):
  the demo reported an unimplemented RPC; independent minimal replay pending.
- [#5 Daemon stop permission diagnosis](https://github.com/leotrax3d/freeRGB/issues/5):
  stopped daemon reported as running; read-only fixture replay pending.

Initial independent triage: #1 needs a clean-runtime environment; #2 did not
reproduce in fresh latest-demo state and stays open for the installed/reinstall
report. Do not close that report based only on source/demo evidence.

The explorer visited all ten views in EN and DE at 1600x1000 and 1024x700:
704 recorded attempts (568 exercised, 100 explicitly omitted, 16 failed and
20 onboarding advances). Each session completed onboarding and matched all
navigation headings. It produced 65 heuristic suspects, not 65 proven bugs.
The four failed cooling controls per session were descendants of closed details;
the final script now crawls summaries and checks ancestor visibility. It also
scans after every action to catch transient errors. These final improvements
await post-fix replay. The bounded crawl does not prove every control combination
or controls left behind after navigation. Local artifacts remain external.

Desktop gate: production build and demo build passed; npm tests passed 31 files,
148 tests. Node syntax and `--help` checks passed. No Rust source was changed by
the harness. The native test GUI, daemon and tray were stopped after the test;
setup completion was restored. Shutdown itself exposed issue #5.

Pipeline: harness ready for coordinator integration; further independent triage,
confirmed fixes and post-fix replay pending. Counts are not final.

| Found | Filed | Confirmed | Fixed | Open |
|---|---|---|---|---|
| 65 automated suspects + native/domain reports | 5 | pending | 0 this cycle; 1 existing fix verified | 4 |

Integration must be performed by the coordinator because the task forbids this
lead from modifying the main checkout. No gate or merge is claimed yet.
