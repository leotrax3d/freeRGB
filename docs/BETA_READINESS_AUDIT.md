# Beta readiness audit — final RC, 2026-10-24

## Latest Windows result: unsigned trial installer ready

[Package run 40104663126](https://github.com/leotrax3d/freeRGB/actions/runs/40104663126)
completed successfully for commit `227214138b13de21bb57a52d737b41278f0e1622`.
[Download the installer artifact](https://github.com/leotrax3d/freeRGB/actions/runs/40104663126/artifacts/12565131407).
No release was created or published; the draft-release job was skipped.

- [x] Native Windows workspace tests, frontend/Tauri tests and release builds.
- [x] Synthetic cooling faults, protected broker boundaries, SCM install/update/rollback/removal with a compile-time sensor fixture; no real hardware access.
- [x] Official prerequisite download/hash/signature verification; no prerequisite installer or driver execution in that check.
- [x] Inno compiler/model checks, actual machine-wide installer install/reinstall/uninstall, legacy refusal, scoped shutdown and preserved profiles on the disposable Windows runner.
- [x] IPC/virtual rendering/plugin smoke and installer artifact checksum/provenance checks. Downloaded artifact was independently reverified locally.
- [ ] Interactive UAC allow/cancel, real cross-version application upgrades and public signed module catalog availability remain open. This is a trial installer, not public-release approval.

Installer: `freeRGB-0.1.0-beta.2-windows-x86_64-setup.exe`, Authenticode `NotSigned` as approved.
SHA-256: `77f44f23f219ccdb576f55552212f6e0eab5da58d8d864a5f761ea322936ef06`.

Build fixes: official ISCC PE resources contain version `0.0.0.0`; the pinned
6.7.3 engine is now checked by its actual preprocessor version in a no-output
probe, while archive hash and Authenticode checks remain mandatory. A missing
Marketplace match arm in the newly merged Win32 GUI was corrected. Compiler
setup now fails early, failed builds retain dependency caches, and the verified
installer artifact uploads before the lifecycle checks.

## Earlier audit snapshot

The record below predates this successful Windows build. Its statements that the
installer was not built/tested are superseded by the results above; the earlier
Linux counts are historical and are not new measurements of the expanded workspace.

## Decision: NO-GO for public 0.1.0-beta.2

The technically reproducible blockers found in source were corrected. Linux release
builds, simulated cooling faults, real local IPC/Marketplace integration and both
GUI startups pass. **The changed Windows installer has not been built or exercised
on native Windows**, including UAC, protected service permissions, cancellation,
upgrade and uninstall. The public `modules` release returns HTTP 404. These two
acceptance gates prevent an honest public-beta GO today. During the audit no push, release, workflow dispatch, tag replacement, physical
hardware access or driver installation occurred. The owner subsequently authorized
commit/push and Windows validation, while release publication remains prohibited.

The existing `v0.1.0-beta.1` tag points to `eb1184a54bdacaebcc18d0f1e92a87712f099edd`.
Its historical [Package run 40007563481](https://github.com/leotrax3d/freeRGB/actions/runs/40007563481)
failed because the Tauri `--locked` build needed a lock update; installer steps were
skipped. It does not validate this candidate. Local application versions and release
notes now consistently use **0.1.0-beta.2**; that candidate has not been published.

## RC corrections

| Risk | Correction and evidence |
| --- | --- |
| High: GLib iterator undefined behavior, RUSTSEC-2024-0429 | Vendored the checksum-verified glib 0.18.5 source and backported only the official iterator fix from upstream PR gtk-rs/gtk-rs-core#1343, commit `05dff0ee696f9bcd8617cd48c4b812d046d440cb`. Both locked graphs select the local patch. Optimized regression exercises all five affected operations, mixed directions, Unicode, empty/exhausted inputs. Original registry source segfaulted in the isolated optimized reproducer; patched source passes. No advisory ignore, ABI migration or fictitious version bump. Provenance/licenses are in `vendor/glib/FREERGB_BACKPORT.md`; CI checks exact source hash and lock selection. |
| High: updater could not elevate administrator installer | Both GUIs call the shared verified `ShellExecuteW/runas` launch path. Cancellation is an error. The verified Windows handle prevents rewriting/deleting the file through the synchronous launch handoff. Normalized DOS/UNC paths support Unicode/spaces. Cross-compiled Windows tests verify sharing restrictions under Wine; actual UAC remains untested. |
| High: elevated execution of user-controlled uninstall commands | Legacy per-user freeRGB blocks machine-wide setup before file changes; Windows Apps removal is required. Competitor uninstall UI refuses HKCU commands under elevated setup. No user registry command is automatically elevated. New native lifecycle test uses a harmless marker executable and asserts refusal; it is not run here. |
| High: installer shutdown/permissions | Removed broad tray `taskkill`; helper checks full installed executable path and captures the process handle before terminating, with disposal and timeout. Missing original-user SID fails closed instead of silently granting a different elevated user access to the service. Mock process test covers unrelated processes and failure paths. |
| Medium: beta updates could never be offered | Beta clients select the highest eligible SemVer from the same prerelease channel or promote to stable; stable clients stay stable. Invalid newest asset metadata fails closed. HTTPS-only API/downloads and existing size/hash checks remain enforced. |
| Medium: ambiguous silent mainboard opt-in | Only the exact case-insensitive `/WITHMAINBOARD` token enables preparation; prefixes, paths and `=0` do not. Installer Pascal model cases added; native compiler execution remains required. |
| High: unusable public module trust configuration | Restored the existing owner's public Ed25519 key, byte-for-byte from repository commit `d3ed610b613d4d66a97918a39129e2c45c2fbc10` (not a generated key). Key `23469f6bec07ca5df93240261eaf3015b2ca8dbe37226e201499d1a8c1823c4a` is compiled in and regression/preflight require a trust anchor. No private key was accessed. Signed public catalog availability is still a separate gate. |
| Release: inconsistent version/toolchain/artifact provenance | Pinned Rust 1.99.1, Node 24.19.0, action commits and Inno 6.7.3 with checksum/Authenticode verification. Preflight enforces version/tag/notes/source metadata parity; build JSON records source commit/dirty state, compiler and all nine payload hashes. Exact artifact set and clean matching source are checked before draft creation. Test installer filename is derived from workspace version. |
| Release: source archive was not independently buildable | Included `packaging/windows/mainboard-packages.json`, aligned module workspace metadata and normalized the subset lock offline without changing package identities/checksums. Exported source builds `--release --locked --offline` after fresh extraction. Two normalized source ZIPs have identical SHA-256; executable reproducibility is not claimed. |
| Release: module workflow supply chain/input | Pinned actions/toolchain/Node in the module workflow too; reject unsupported module identifiers before shell/build steps, and pass the validated environment value rather than inserting dispatch text into shell source. |

## Executed RC validation

Linux builds used Debian 13, Rust 1.99.1 and Node 24.19.0 in a container without
hardware/device passthrough. Only checks relevant to RC changes were repeated;
the earlier complete suite is retained separately below.

| Check | Actual result and scope |
| --- | --- |
| Targeted Rust tests | **95 passed**: modules 9, updater 5, daemon 73, cooling-guard process fixtures 8. Faults include killed/hung producers, stale/critical temperatures, stalls, maintenance and oversized/blocked input; synthetic duty sinks only. |
| Tauri Rust tests | **11 passed**, including update serialization, fixture and polling behavior. |
| Optimized GLib regression | **1 passed**, release profile; original unpatched isolated repro segfaulted as expected. Source comparison: 121 files, only `src/variant_iter.rs` code differs. |
| Windows updater | GNU cross-compilation passed; **6 passed under Wine**, including Unicode/spaced paths, exclusive staging, pre-launch verification and Windows file-sharing denial. No installer execution or UAC assertion. Wine initially lacked UTF-8 locale for the new path fixture; rerun with `LANG/LC_ALL=C.UTF-8` passed. This is compatibility evidence, not native Windows acceptance. |
| Workspace/Tauri Clippy | Both all-target runs with `-D warnings` passed; focused updater rerun also passed. Vendored old GLib emits 34 upstream style/lifetime warnings as a dependency; these are documented, not suppressed. |
| Release builds | `cargo build --workspace --release --locked` and Tauri `--release --locked --features tauri/custom-protocol` passed. Release binaries were used for the integration checks below. |
| Desktop | Production build passed with beta.2 metadata; **87 tests passed** across 19 files. |
| Linux release integration | Real daemon Marketplace smoke passed: file package install, render, restart persistence, pinned update, uninstall and corruption recovery. Classic/Tauri started under Xvfb with virtual daemon and API color round trip. No physical devices or real remote marketplace exercised. |
| Security | Fresh cargo-audit database commit `21d909e4c8bd4e14c7aea58bf980c881a0053ab1`: **0 vulnerabilities, 0 unsound warnings** in both graphs. Main graph: 6 unmaintained warnings; Tauri: 1. Audit does not establish safety of arbitrary local patches; backport integrity/regression checks supply separate evidence. |
| Prerequisite downloads | All **7** manifest payload/notice size+SHA checks passed without execution. Linux osslsigncode matched PE digests but chain verification failed for missing local code-signing/timestamp roots. **No Authenticode success is claimed**; Windows strict verification remains mandatory. |
| Module source | Frozen subset contains 28 original package identities; release locked build and fresh-extraction offline build passed. Two exports SHA-256 `39ac721ebc2092666527f38ed3e4f56edea26b55f5805ab7e69ed27ec6a9804d`. |
| Static/preflight | Metadata, exact GLib backport, two release-preflight tests, workflow actionlint, all PowerShell AST parsing, mock installer process scope, Rust formatting and `git diff --check` passed. Actionlint ran without optional shellcheck/pyflakes. |
| Windows dependency resolution | Tauri Windows MSVC metadata with custom-protocol resolves under `--locked`. This is not an MSVC compilation. |
| Inno compiler/model/lifecycle | **Not validated**. Pinned official 6.7.3 installer hash verified; Linux/Wine lacks usable 32-bit execution, and alternate 64-bit compiler setup rejected Wine. Neither successful installer compilation nor Pascal-model/lifecycle execution is claimed. Native Windows is required. |

## Public-beta acceptance checklist

- [x] Correct GTK unsoundness; verify exact backport and optimized regression.
- [x] Correct updater beta selection/elevation and installer privilege/shutdown issues.
- [x] Build release binaries, retest affected logic and local component integration.
- [x] Align beta.2 versions, lockfiles, toolchains and artifact/source provenance.
- [ ] **Native current-candidate Windows gate:** compile pinned installer; run Pascal models, synthetic native adapter/broker/journal/service tests, lifecycle install/reinstall/removal, legacy refusal, permission boundaries, UAC allow/cancel, failed dependency downloads, interrupted upgrades and cross-version update/rollback on disposable Windows. Historical CI tests of beta.1 do not close this gate. Commit/push and subsequent Windows validation are now authorized; results will be recorded when actually available.
- [ ] **Public modules gate:** publish/verify the owner-signed catalog and matching packages/source for the compiled trust anchor. Anonymous `releases/tags/modules` returned 404; mock signing tests and restoring the key do not prove production availability. No module release was created here.
- [x] **Signing decision:** the owner explicitly approved an unsigned initial beta. No freeRGB publisher Authenticode certificate is configured; SmartScreen warnings remain possible. Checksums/build metadata are not publisher signatures. Module-catalog signatures and prerequisite Authenticode checks remain mandatory.
- [ ] **Native cooling limits:** simulation validates decisions, not physical speed, driver hangs or independent physical recovery. Keep experimental/support-tier gates and do not represent requested 100% duty as verified fan response. No real hardware testing was attempted, as instructed.
- [ ] **Service/operational limits:** production Appwrite/auth/CDN acceptance, prolonged load/CPU/RAM measurement and interactive Windows accessibility remain unverified. Six main Rust unmaintained packages (`paste`, `proc-macro-error`, `ttf-parser`, `zbus_names`, `zvariant`, `zvariant_derive`) and Tauri `proc-macro-error` require upstream tracking. Rust 1.90 MSRV and Node 22 compatibility were not executed; pinned release tools are the tested versions.

## Evidence and reproduction

Local logs and generated artifacts are under ignored `target/rc-audit/`; earlier
browser/GUI evidence is in `target/quality-audit/`. They describe this checkout,
not a published installer. See CI/Package workflows for native Windows commands;
`test-installer.ps1` refuses destructive lifecycle tests on personal machines.
Run `check-release.py`, `test-release-preflight.py`, `check-glib-backport.py` and
`test-installer-process-scope.ps1` before packaging. Linux native smoke uses
`scripts/test-native-gui-linux.py <release-binary-directory>` with its private
virtual daemon. No current-candidate Windows pass is available to attach.

## Previous comprehensive audit — historical baseline

The following records checks completed before the focused RC work. Counts are
preserved for traceability; they are **not a claim that the entire suite was
repeated after every RC edit**. The GLib warning and development-only build limits
in that baseline were superseded by the RC results above.

## Fixed findings

| Priority | Finding and correction |
| --- | --- |
| High | Clock reversal could hold a low rate-limited fan output. `CurveController` now immediately requests 100%, then resumes rate limiting from that fallback. Pure policy regression; no physical write is claimed. |
| High | Tray fan actions could appear unlocked when the safety RPC reply was missing. Missing consent now fails closed. |
| High | Known dependency advisories: updated `rustls-webpki` 0.103.15 → 0.103.16, `webpki-roots` 1.0.9 → 1.0.11, both direct ZIP dependencies 2.6.1 → 9.0.5, and Tauri's Tokio 1.54.0 → 1.54.1. ZIP filename decoding errors are propagated through the new API. |
| High | Package extraction trusted an empty destination without enforcing it and used overwriting file creation. It now rejects nonempty/linked destination directories and exclusively creates output files. Tests preserve existing data and reject archive symlinks. |
| Medium | A second Unix daemon could unlink a live socket, with a race during stale-socket recovery. A lifetime OS file lock serializes admission/recovery; live sockets, ordinary files and links are preserved. API startup failures now produce a failing process exit status after cleanup. |
| Medium | Update downloads shared a truncating `.part` file. Each operation now reserves a separate file with exclusive creation. Tauri update actions reject concurrent check/download/install operations before network or execution. |
| Medium | Tauri treated explicitly disabled modules as a disconnected daemon. Feature RPC errors now clear the affected slice without declaring a transport failure. Native startup reproduced and verified this correction. |
| Medium | Setup could disappear or advance after a failed completion RPC. Both wizard levels retain errors and allow retry; onboarding remains visible while saving. Browser regression injects both failure cases. |
| Medium | Curve validation accepted non-finite values; macro validation accepted negative/fractional delays and non-finite brightness. Client validation now rejects them before serialization. The daemon remains authoritative. |
| Medium | Null-rating deletion bypassed community blocking/quotas. All rating mutations now use the same quota check. Invalid JSON returns 400 instead of a misleading 503. Mock backend assertions cover blocked deletion, exhausted quotas and malformed input. |
| Medium | Website selector parser was affected by GHSA-rj75-hqrm-r3gf. A targeted override updates it to the patched 7.x line; the complete site/demo/admin build and npm audit pass. |
| Build | Classic GUI required `libxkbcommon-x11`, absent from the documented/CI dependency list. Added the development package and verified native startup after installation. |
| Build | Rust 1.80 was advertised although locked dependencies already require newer Rust, including Tauri 1.90. Manifests/README now state 1.90; the tested compiler is 1.99.1. Mechanical Clippy fixes use APIs available at the corrected minimum. Rust 1.90 itself was not tested. |

The architecture and existing feature scope were retained. Community mock tests
were added to CI. New browser/native smoke scripts are available under `scripts/`.

## Executed validation

Final Linux checks used Debian 13, Rust 1.99.1 and Node 24.19.0. Native Rust builds
ran in an isolated Docker container with the repository's Linux dependencies,
four build jobs and no hardware/device passthrough. Debug information was disabled
to bound build storage. Package checksums, TLS and package signatures stayed enabled.

| Check | Final result |
| --- | --- |
| `cargo test --workspace --locked` | **276 passed, 0 failed, 1 ignored**. The ignored module test was subsequently executed separately. Includes slow/failing device workers, plugin hangs/crashes, protocol bounds, persistence and sandbox/fuel tests. |
| `cargo test -p freergb-daemon --features cooling-fixture --test cooling_guard_process --locked` | **8 passed**: hung/killed producers, blocked/oversized input, stale readings, critical temperature, stall and maintenance recovery. Synthetic duty sinks only. |
| `cargo test -p freergb-modules --test e2e --locked -- --ignored` | **1 passed**: signed catalog, installed module process and simulated Cobra. The ignored test is not left unexecuted. |
| Tauri `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked` | **11 passed**, including fixtures, update serialization and disabled-feature polling. |
| Workspace and Tauri Clippy, all targets, `-D warnings` | Passed. Initial lint failures were corrected; no warning suppression was introduced. |
| Workspace build and Tauri embedded build (`tauri/custom-protocol`), locked | Passed in the development profile. No release-profile rebuild or Windows build is claimed. |
| Workspace/Tauri formatting; `git diff --check`; README progress check | Passed. |
| Desktop `npm test` | **87 passed** in 19 files. |
| Community `npm test` | **1 integration scenario passed**, covering identity isolation, admin privacy, retry deduplication and quota concurrency, with added failure assertions. Uses a mock backend. |
| Desktop production build, demo build, website and admin build; combined `build:site` | Passed. Combined build's native/network API isolation audit passed. Website has no standalone unit-test command. |
| `scripts/test-effect-marketplace.py` against actual Linux daemon | Passed: duplicate daemon rejection, IPC, local-file package download/install, rendering, persisted restart, pinned update, uninstall and corruption recovery. No external download or real installer execution. |
| `scripts/test-desktop-browser.mjs` | Passed: failed device completion, failed onboarding completion, successful retries and all nine views; no JavaScript page errors. Mock transport is asserted before interaction. |
| Built website/demo browser checks | Desktop/mobile website rendered without horizontal overflow at 1440/390 px; built demo setup and Lighting worked with no external demo requests or JavaScript errors. Website remote requests were deliberately blocked to test bundled fallback. |
| `scripts/test-native-gui-linux.py` | Classic and embedded Tauri started under Xvfb with a private virtual daemon; API color round trip passed. Screenshots were inspected. This is startup/connection evidence, not automated native control clicking. |
| `npm audit` for desktop, website and admin | **0 reported vulnerabilities** in each final lockfile. |
| `cargo audit` for both Cargo lockfiles | **0 vulnerability findings** after corrections. Informational/unsoundness warnings remain below; the result is not a clean bill of health. |

Tests and builds were repeated after corrections. Early failures included missing
system libraries, ZIP 9's changed filename API, newly applicable Clippy lints,
the native Classic X11 dependency and Tauri's false offline state. Their relevant
checks were rerun successfully. A browser navigation attempt initially encountered
the expected first-use tour; the final test explicitly closes it before navigation.

