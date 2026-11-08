# Non-commercial repository check

Reviewed 2026-11-08 on branch `task/legal-beta`. Owner instruction: publisher is
one private natural person; project is strictly non-commercial. Community/online
features, telemetry and download counting must remain off by default. This is a
repository inventory, not a determination of legal applicability or approval.

## Scope and method

Searched tracked repository text including hidden `.github` configuration,
README, website, docs, desktop/classic app sources, installer, Appwrite and
Supabase with case-insensitive terms: donation/donate, sponsor, funding,
Patreon, Ko-fi, PayPal, Buy me a coffee, commercial. Also checked German
Spenden/Sponsoring and payment/advertising terms. A second tracked-file `git grep` pass included module sources read-only. Dependency lock files and
third-party licence language are not project solicitations. No module code changed.

## Findings and disposition

| Location | Finding | Disposition |
| --- | --- | --- |
| `apps/admin/package-lock.json`, `apps/desktop/package-lock.json`, `website/package-lock.json` | Third-party dependency funding metadata (including upstream sponsor destinations) | Flagged and retained as dependency metadata; not publisher fundraising. npm may print its upstream funding notice during installation |
| `.github` | No tracked FUNDING.yml or FUNDING.yaml; no project funding solicitation found | No removal needed |
| README, website, app, installer, Appwrite, Supabase | No donation, sponsor, payment or commercial offer link found | No removal needed; external destinations and future content require a new review |
| `LICENSE-APACHE:163` | Commercial damages in upstream licence terms | Retained; licence language, not a commercial offer |
| `docs/legal/OWNER_DECISIONS.md`, `LAWYER_REVIEW.md`, `PERSONAL_LIABILITY.md`, `RISK_REGISTER.md`, `RISK_REGISTER.json` | Funding, sponsoring, sales and operator questions in review material | Flagged: historical questions; owner now specifies private individual and non-commercial operation. Do not treat as offers or resolved legal conclusions |
| `packaging/windows/freergb.iss` | Publisher was generic contributors | Removed generic publisher field; exact private publisher identity remains a release integration item |
| README, app startup/sidebar/About, website, installer | Beta wording incomplete or implicit in version only | Added explicit visible Beta text without a new design |
| `docs/USER_GUIDE.md`, desktop README, beta.1 release notes, website content | Unqualified reassurance wording | Replaced with factual read-only/mock/guided descriptions |
| README fan-control table | Floors described as always on despite unrestricted mode | Qualified by backend and unrestricted settings |

Legal review records and archived engineering plans retain terminology as
historical evidence; they are not current product assurances. Existing policy
identifier names and fallback terminology in source are not product claims.
Flagged for the owning package: `crates/freergb-daemon/src/gpu_cooling.rs:1`
still describes guaranteed driver return, exceeding what recovery attempts
establish. That file is reserved by the release plan and was not edited.

No new liability/disclaimer wording was integrated; `SAFETY_LIABILITY_TEXTS.md`
is owned by another worker.

## Open points

- Integrate the private publisher's chosen identifying details in a separate
  release pass, together with the other worker's disclaimer draft.
- `OWNER_DECISIONS.md` / release-plan reconciliation belongs to the lead.
- Offline defaults: `crates/freergb-effect-host/src/ratings.rs` defines
  `CommunitySettings` boolean defaults off; `counting_enabled` requires both
  online and counting. This does not establish every telemetry/update path's
  behavior. Privacy package must verify the complete default configuration and
  network behavior; website external fonts remain an existing privacy item.
- No external payment service/account, hosted content or deployed backend was
  inspected or changed. Recheck release artefacts and deployed pages separately.
