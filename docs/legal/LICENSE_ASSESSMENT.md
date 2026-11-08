# Licence assessment: permissive core + GPL process modules (DRAFT)

> **DRAFT – AI assessment, not legal advice.** Written by an AI assistant from
> the repository contents at the time of writing. It is not reviewed or approved
> by a lawyer and makes no claim that any licence obligation is met. Every
> conclusion below is a working hypothesis for the questions in section 8.
> Related: `docs/legal/OWNER_DECISIONS.md` (D2, D3), `docs/legal/RELEASE_FIX_PLAN.md`
> (FR-007, FR-010, FR-011), `docs/legal/LICENSE_AUDIT.md`, `docs/legal/LAWYER_REVIEW.md`.

## 1. Owner context (given, not verified here)

- Publisher: one private natural person.
- Strictly non-commercial: no sales, no paid features, no paid support, no ads,
  no data monetisation.
- Online/community features OFF by default; telemetry and download counting OFF
  by default (see D4/D8 in `OWNER_DECISIONS.md`).

Non-commercial distribution does **not** remove GPL or MIT/Apache obligations;
those licences attach to distribution as such, not to payment.

## 2. Licence layout as found in the code

| Part | Declared licence | Evidence |
|---|---|---|
| Workspace default (all `crates/*`) | `MIT OR Apache-2.0` | `Cargo.toml:131`, `license.workspace = true` in each `crates/*/Cargo.toml`; `LICENSE-MIT`, `LICENSE-APACHE` |
| Desktop app | `MIT OR Apache-2.0` | `apps/desktop/src-tauri/Cargo.toml:6` |
| Hardware modules `modules/<id>/` | `GPL-2.0-or-later` | e.g. `modules/a4tech/Cargo.toml` (SPDX header + `license=`), `modules/a4tech/COPYING`, `modules/a4tech/module-info.json` |
| Module origin | OpenRGB at a pinned commit | `modules/<id>/PROVENANCE.md`, `modules/<id>/source-workspace.toml` (e.g. a4tech: commit `bbc77470…`) |

Relevant crates:

- `crates/freergb-module-api` – protocol types and module SDK (`src/lib.rs`,
  `src/sdk.rs`, `src/transport.rs`, `src/bus.rs`, `src/cooling.rs`). Licensed
  MIT OR Apache-2.0. **Statically linked into every GPL module** (module
  `Cargo.toml` depends on it via path). Deps: `freergb-core`, `serde`,
  `serde_json`, `base64`.
- `crates/freergb-modules` – host side: process spawn (`src/host.rs`),
  transport/bus/cooling brokers (`src/transport_host.rs`, `src/bus_host.rs`,
  `src/cooling_host.rs`), catalog/download/signature checks (`src/catalog.rs`,
  `src/package.rs`, `src/store.rs`, `src/trust.rs`, `src/source.rs`).
- `crates/freergb-module-tool` – builds module zips and source zips, catalog
  entries (`src/main.rs`, `src/release.rs`).
- `crates/freergb-daemon` – depends on `freergb-modules` and
  `freergb-module-api` only (`crates/freergb-daemon/Cargo.toml:42-43`); a grep of
  `crates/*/Cargo.toml` found **no** core crate depending on a `modules/*` crate.
  `crates/freergb-daemon/src/mainboard.rs` also spawns a child process over
  stdin/stdout (mainboard broker) – same pattern, to be checked per binary.

## 3. The broker / process boundary

What the code does (`crates/freergb-module-api/src/lib.rs` header,
`crates/freergb-modules/src/host.rs:101-150`, `docs/HARDWARE_MODULES.md`):

- Each module is a separate executable, started with `Command::new`, talking
  UTF-8 JSON lines over piped stdin/stdout, max message size `MAX_MESSAGE`.
- No shared Rust ABI, no dynamic loading, no shared memory found.
- The daemon owns device handles; the module requests report transfers
  (`send_feature` etc.) via the broker.
- Modules are downloaded on request and verified (ed25519, sha256), not
  shipped inside the installer (no module entries found in
  `packaging/windows/freergb.iss`).

AI assessment:

- **Direction permissive → GPL is unproblematic in principle.** MIT/Apache-2.0
  code (`freergb-module-api`, `freergb-core`, serde …) linked into a
  GPL-2.0-or-later binary is the common case; Apache-2.0 compatibility relies on
  the "or later" (GPLv3) option or on choosing the MIT alternative. The module
  binary as a whole is then distributed under the GPL.
- **Core ↔ module:** separate executables communicating over a narrow pipe
  protocol are the pattern the FSF GPL FAQ ("aggregate" vs. "combined work")
  describes as *usually* separate programs, but it also says that intimate
  communication exchanging complex internal data structures may make them one
  program. Indicators here pointing to *separate*: own process, JSON protocol,
  documented versioned API (`MODULE_API`), modules optional and downloaded
  separately, core works without any module. Indicators a lawyer may weigh
  the other way: the protocol was designed by the same author for these modules;
  modules are useless without the daemon; the daemon UI presents module devices
  as one product; one catalog/one signing key.
- Under German/EU law the question is framed as "Bearbeitung" / derivative
  work (§ 3, § 23 UrhG, Art. 4 Software Directive); there is no settled case law
  on process boundaries. Whether the FSF view carries over is a lawyer question.
- **Open item that the boundary does not cover:** any OpenRGB-derived code
  *inside the permissive core*. `crates/freergb-protocols/src/registry.rs:55`
  cites "OpenRGB reference list"; D3 (provenance of in-tree protocol code) is
  unresolved. If core files are GPL-derived, the process boundary does not help
  for those files.

## 4. Obligations for binary releases (as understood, unverified)

For each published module binary `freergb-module-<id>-<version>-<target>.zip`:

1. Licence text (GPL-2.0 `COPYING`) in the zip – `release.rs` `pack` takes
   `files` incl. `COPYING`, `PROVENANCE.md` (fixture in
   `crates/freergb-module-tool/src/release.rs:321`).
2. Complete corresponding source, incl. build scripts: produced by
   `.github/workflows/modules.yml:256-278` as
   `freergb-module-<id>-<version>-source.zip`, re-built from the archive itself
   ("Source archive builds on its own"). `release.rs` enforces that a `source`
   step precedes every `add` (`release.rs:81-87`).
3. Source offered "equivalently" next to the binary (GPLv2 §3(a)) or a written
   offer (§3(b), 3 years). Draft text: `SOURCE_OFFER.md` on branch
   `origin/task/legal-notices` (not on this branch; contact address placeholder).
4. Notices for permissive deps inside the module binary (MIT/Apache attribution,
   Apache NOTICE if any) – FR-007, `THIRD_PARTY_NOTICES` still todo.
5. Prominent modification notices (GPLv2 §2(a)): changes vs. OpenRGB are
   described in `PROVENANCE.md`; whether this suffices is open.
6. Only allowlisted IDs build for release: `packaging/modules/release-allowlist.txt`
   (currently only `razer-chroma`). No module release has been published yet
   (`LICENSE_AUDIT.md`), so no real binary/source pair has been checked.

For the core installer: MIT/Apache notices for all bundled deps (FR-007), plus
separate handling of PawnIO and its LGPL-2.1 modules (`LICENSE_AUDIT.md:12`).

## 5. Risks (AI view, unweighted by a lawyer)

| # | Risk | Why | Mitigation in repo / open |
|---|---|---|---|
| R1 | Core considered combined with GPL modules | Same author, purpose-built protocol, single product UX | Process boundary; lawyer opinion needed (D2) |
| R2 | GPL-derived code in permissive crates | Ports "after viewing sources", registry cites OpenRGB | D3 provenance review per file; open |
| R3 | Source zip not "complete corresponding" | Missing lockfile/vendor/build scripts | CI self-build check; Cargo.lock coverage to verify |
| R4 | Binary and source drift (different commit) | Separate jobs | Same workflow, same commit; needs release-SHA check (D7) |
| R5 | Incomplete attribution of OpenRGB authors | `PROVENANCE.md` lists files, not all copyright holders | Open (FR-010) |
| R6 | Missing notices for permissive deps in module zips | Only COPYING/PROVENANCE packed | FR-007 open |
| R7 | Apache-2.0 + GPL-2.0-only mix | Any upstream file that is GPL-2.0-*only* | Verify SPDX of every used upstream file |
| R8 | Written offer unenforceable without contact | Placeholder in draft | Owner must supply address |
| R9 | Relicensing core later | Contributors' rights | No CLA found; decide before external contributions |

## 6. What this document does not establish

- No licence compliance, no legal approval, no clean-room claim.
- No check of any actual released artefact (none exist yet).
- No review of every one of the modules under `modules/`; only representative
  files were read.

## 7. Suggested engineering follow-ups (not done here)

- Add MIT/Apache notice files for linked deps to each module zip (FR-007).
- Include `Cargo.lock` in each source zip and document it in the source offer.
- Merge `SOURCE_OFFER.md` from `task/legal-notices` after owner fills placeholders.
- Per-file provenance list for `crates/freergb-protocols` (D3).

## 8. Exact questions for a lawyer

1. Under German/EU copyright law, are `freergb-daemon` (MIT OR Apache-2.0) and a
   module executable such as `modules/razer-chroma` (GPL-2.0-or-later), which
   communicate only via the JSON-line pipe protocol in
   `crates/freergb-module-api/src/lib.rs`, separate works, or does distribution
   make the daemon subject to the GPL?
2. Does it change the answer that modules are downloaded separately on user
   request (not in the installer), or that one catalog and signing key cover both?
3. May `freergb-module-api` stay MIT OR Apache-2.0 while statically linked into
   GPL modules, and which notices must the module zip carry for it?
4. Is the source zip produced by `.github/workflows/modules.yml` (module sources,
   workspace files, licence texts, rebuilt from itself) "complete corresponding
   source" under GPLv2 §3? Must `Cargo.lock`, toolchain pin (`rust-toolchain.toml`)
   and CI scripts be included?
5. Is publishing the source zip next to the binary on GitHub Releases sufficient
   (§3(a)), or is the written offer in `SOURCE_OFFER.md` additionally required?
   What contact data must a private person publish?
6. Do `PROVENANCE.md` files satisfy the §2(a) modification notice and copyright
   notice duties for OpenRGB-derived code, or must individual authors be listed?
7. For protocol knowledge (byte sequences, device IDs) taken from OpenRGB and
   re-implemented in permissive crates (e.g. `crates/freergb-protocols/src/registry.rs`):
   when is this an unprotected idea/interface (Art. 1(2), Art. 6 Software
   Directive; § 69a(2) UrhG) and when a derivative of GPL code?
8. Does being a strictly non-commercial private natural person change any
   obligation or liability exposure for GPL/MIT/Apache distribution?
9. Should the project keep MIT OR Apache-2.0 for the core, or would GPL for the
   whole project materially reduce risk R1/R2? Any downside for contributions?
10. What happens if a single upstream file used by a module is GPL-2.0-only
    (combination with Apache-2.0 deps)?
