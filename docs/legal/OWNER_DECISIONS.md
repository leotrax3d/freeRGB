# Owner and lawyer decisions

Engineering cannot settle these. Each item names the decision, the engineering
artefact that is ready (or will be), and what is blocked until it is decided.
Nothing here is legal approval; wording drafts are proposals for a lawyer to
review. Related: `LAWYER_REVIEW.md`, `RELEASE_FIX_PLAN.md`.

| # | Finding | Decision needed (owner / lawyer) | Prepared by engineering | Blocks |
|---|---|---|---|---|
| D1 | FR-016 | Who is publisher/manufacturer/operator (person or entity)? Funding, sponsoring, sales, OEM? Target markets? | Applicability matrix draft (CRA, product liability, consumer law) with the questions to answer | Any public release, liability text |
| D2 | FR-007/015 | Project licence stays MIT/Apache-2.0 core + GPL-2.0-or-later modules? Is the broker/module boundary a separate-works boundary? Lawyer opinion on the concrete files | NOTICE, THIRD_PARTY_NOTICES, SBOM, source-offer text, module provenance report, file list for the lawyer | Shipping modules, installer text |
| D3 | FR-015 | Origin of in-tree protocol code written after viewing other sources; keep, rewrite or remove per file | Provenance gap report; no clean-room claim is made anywhere | Release of affected protocol crates |
| D4 | FR-006 | Controller of the community service, purposes, legal basis (Art. 6 GDPR), retention, processors (Appwrite DPA, location), whether to run the online service at all | Privacy notice draft, data-flow table, offline/no-count switch in code, retention settings | Enabling online marketplace in the default config |
| D5 | FR-005 / hardware liability | Final wording of safety text, disclaimer, German version; insurance (product/professional liability) | Corrected claims per backend with text version; EN/DE draft text | Public release |
| D6 | FR-017 | Rights chain for logo, fonts, figma assets; trademark search for "freeRGB" in target markets; patent review of implemented functions | Asset provenance checklist, search task list | Name/branding in public release |
| D7 | FR-018 | Accept release scope (source-only, beta with hardware limits, full)? Name the release SHA | Release-SHA checklist, artefact verification steps | GO decision |
| D8 | P0.4/P0.5 | Update/telemetry policy: default offline? Download counting opt-in? | Config switch; default proposal: counting off until D4 settled | Default settings |
| D9 | FR-001..004, hardware | Hardware acceptance with the owner present, release key, signing, publishing | Steps in `docs/promts/fancontrol/07-hardware-acceptance.md` | Hardware release claims |

## Default proposals (engineering, pending decisions)

- Ship community features disabled by default until D4 is answered.
- No statement of "safe", "certified" or "patent-free" in any text.
- Safety text claims only what a test or the code proves for the given backend.
- Do not change the licence or send any legal position to third parties without
  a separate explicit owner decision.

## Owner answers (2026-11-08)

Recorded as the owner's decisions. Nothing here is legal approval.

| # | Decision | Implementation |
|---|---|---|
| D1 | Publisher is one natural person (private individual). No company, no sales. | Texts written for a private, gratuitous publisher |
| D2 | Licence assessment by an AI worker, labelled as such | `docs/legal/LICENSE_ASSESSMENT.md` (task/legal-license) |
| D3 | Keep files with clean provenance; rewrite unclear ones without reading upstream source (only freeRGB protocol docs/tests) or remove; record per file | Provenance gap report; rewrite package after notices is merged |
| D4 | Online/community OFF by default, explicit opt-in; controller = owner; consent Art. 6(1)(a) GDPR for accounts/ratings; no download counting by default; minimal retention; Appwrite in an EU region with accepted DPA | FR-006 merged; Appwrite alignment pending (read-only first) |
| D5 | Safety/liability texts EN+DE as final drafts, only code/test-proven claims | `docs/legal/SAFETY_LIABILITY_TEXTS.md` (task/legal-texts) |
| D6 | No trademark/patent search; only ensure bundled assets are own or properly licensed | `docs/legal/ASSET_LIST.md` |
| D7 | Scope proposed by a worker: public beta, binaries + source, hardware limits stated, online off | `docs/legal/RELEASE_SCOPE.md` |
| D8 | Telemetry and download counting off by default | In code (FR-006) |

### Liability requirements (owner)

1. Strictly non-commercial project by a private person: no sales, paid features, paid support, ads or data monetisation. No donation/sponsor links without a separate owner decision; existing ones are removed or flagged (`NONCOMMERCIAL_CHECK.md`).
2. Visible "Beta" label in installer, app About/startup, README and website. No promises such as "safe", "tested for board X" or "protects your hardware".
3. Disclaimer under German law (EN+DE), shown in the installer (must accept) and in the app (About and before enabling fan control). It must not claim an exclusion where German law forbids it (life/body/health, gross negligence, intent); gratuitous-provision rules (§§ 521, 524 BGB) referenced. Draft for lawyer review.
4. Fan control: default OFF and explicit opt-in, minimum RPM/pump protection, BIOS fallback and warnings before unrestricted unlock are verified from code (`FAN_CONTROL_DEFAULTS.md`); gaps become packages or hardware items.
5. The owner checks liability insurance coverage. Optional later: an association or foundation as publisher.
