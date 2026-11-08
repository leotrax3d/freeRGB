# Release candidate 0.1.0-beta.5

Status: **candidate, unpublished.** No tag, GitHub release or upload was made.
Artifacts are staged on the build machine in `E:/freergb-build/release-0.1.0-beta.5`
(not in the repository). Release notes: [RELEASE_NOTES_0.1.0-beta.5.md](RELEASE_NOTES_0.1.0-beta.5.md).

## What changed for the candidate

- Version bumped to `0.1.0-beta.5` (workspace, desktop package/lock/tauri config,
  module source workspace, both Cargo.lock files).
- `PINNED_UPDATE_KEY` in `crates/freergb-update` now pins
  `23469f6bec07ca5df93240261eaf3015b2ca8dbe37226e201499d1a8c1823c4a` (equals
  `crates/freergb-modules/keys/release-keys.txt`). Other keys and unsigned installers
  still fail closed. See [UPDATES.md](UPDATES.md).
- New `freergb-module-tool sign-installer` writes the detached `<installer>.sig`;
  `scripts/check-release.py` and the CI Package workflow do not sign (no key in CI).
- Fixed two build blockers found on the way: a UTF-8 BOM in
  `modules/thermaltake-poseidon-z-rgb/Cargo.toml` broke `check-release.py`, and
  `apps/desktop/src-tauri/Cargo.lock` was stale (`--locked` build failed).
- Ignored release test `staged_release_signature_verifies_with_pinned_key` checks
  a staged installer signature against the really pinned key.

## Built artifacts (SHA-256)

Installer commit `7f05749179c5674002eb3da0773efa51e6830ec3`, clean tree, rustc 1.99.1,
Node 24.20.0, Inno Setup 6.7.3 (pinned installer, in `E:/freergb-build/inno-6.7.3`).

| File | SHA-256 |
|---|---|
| installer/freeRGB-0.1.0-beta.5-windows-x86_64-setup.exe | f0ecdb427f91f41f416fe0c7512a1564fe9af894306d630689cecf28a8eafffa |
| installer/...setup.exe.sig (64 raw bytes) | 9c3422b4ac8972fa989b7978911f0f74d60c8731e73ef5da974a8366e82a9e06 |
| installer/...build.json | 2539c1b6d258ef3b5202cdac8a3e2b386b9528ff72f3a641bb4fb5d3db64abb7 |
| modules/catalog.json | 42f6e1f4632ec6ccaf5206631a21edf406823043aece2d7203b2fdaa9ec66174 |
| modules/catalog.json.sig | 2b148e71877274aea432c0868ed9455628ae05e4731c0cf4438ce2466ad64442 |
| modules/freergb-module-razer-chroma-0.1.0-x86_64-pc-windows-msvc.zip | 1a6daf7474880cd25a9cd7c9adfa3790399b9fcfa2d9127d5ab1bf04131b0d69 |
| modules/freergb-module-razer-chroma-0.1.0-source.zip | 218e524a77217435cca5323750bd3215fb7f629f97bc1662b60a18ee41f602b4 |
| effects-catalog/catalog.json (3 effects, version 1.0.1, sequence 2) | 7a5b1cd0f54680ac9240d30a8a811fed5c1cfdd0bfc86e2e73b3b9cd0537ce72 |

All files, including the three `.wasm` packages, are in `SHA256SUMS` in the staging
directory. The installer size is 18966196 bytes.

## How it was verified

- `python scripts/check-release.py` (metadata) and `--artifacts installer/` passed.
- `cargo test -p freergb-update -p freergb-module-tool --locked`, including the
  ignored test with the real signature: passed. `cargo clippy ... -D warnings` and
  `cargo fmt --check`: clean for the touched crates.
- `cargo build --workspace --release --locked` succeeded.
- Module catalog signed with the existing key and checked with
  `freergb-module-tool verify --pubkey <pinned key>`; the module source archive was
  frozen and built standalone (same steps as `modules.yml`).
- The private key was only passed as a file path to the repo tools.

## What is missing

- The installer was only built, never run (no UAC, install, upgrade or rollback test).
  The Package workflow's installer tests and full `cargo test --workspace` were not
  run here; run them in CI before publishing.
- Not Authenticode-signed (SmartScreen warning).
- Linux packages are CI-only (`packaging/linux/build.sh`, `ci.yml`); none built here.
- The module catalog contains only `razer-chroma` (the only ID on
  `packaging/modules/release-allowlist.txt`). Sequence 1 is correct only because no
  `modules` release exists on GitHub yet; otherwise continue the published catalog.
  Only the Windows package was built; the Linux module package comes from CI.
- Effects catalog is unsigned by design (size/SHA-256 bound). Version `1.0.1`
  and sequence `2` are the workflow defaults; confirm against the published `effects`
  release before uploading. The 25 development effects are not part of it.
- Publishing: the `.sig` must be uploaded next to the installer, but
  `publish-installer.yml` and `check-release.py --artifacts` accept exactly the three
  CI files, so the `.sig` has to be added to the draft release by hand (or the
  workflow extended) after review.
- Source commit of the installer (`7f05749`) precedes the commit adding the ignored
  test and this document; they change no shipped code. Rebuild from the final
  release commit if bit-exact provenance matters.
- Hardware features remain untested, see the release notes.
