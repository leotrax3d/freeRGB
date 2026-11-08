# D4 alignment inventory

2026-11-08: MCP unavailable due to expired authorization. All remote items below
are **unverified**. This is a source comparison, not a live audit or legal approval.

| Area | Repository evidence / expected inventory | Missing remote evidence |
|---|---|---|
| Region | Historical endpoint `fra.cloud.appwrite.io`, project `6adb5259001299138e7b`; owner requires EU | Actual project region, data locations, DPA version/acceptance evidence |
| Database | TablesDB `community`; no legacy collection inventory available | List all databases, tables AND legacy collections; get enabled state, schema and indexes |
| Ratings | `effect_id`, `user_id`, `stars`; deterministic identity/effect ID | Types, required/default values, indexes, permissions |
| Downloads | `effect_id`, `user_id`, `version`; deterministic receipt ID | Same; any historical counting data |
| Submissions | `kind`, `user_id`, `payload`, optional `status` | Same; payload limits and admin read role |
| Messages | `submission_id`, `user_id`, `author_id`, `text`, `internal` | Same; linked-message deletion completeness |
| Quotas | `expires` epoch seconds; hashed slot ID | Type/index, real expiry backlog (counts only, no row content needed) |
| Audit / blocks | `actor`, `action`, `target` / `reason`, user ID as block ID | Schema, permissions, existing expiry mechanism |
| Permissions | Setup says row security off, no client rights except admin-team submission read; source creates rows with empty permissions | Get table/collection permissions and row/document-security settings; team membership roles without exporting member personal data |
| Auth | Historical anonymous auth, persisted native cookie, fresh JWT per action | Enabled methods/providers, session duration/limits, abuse controls, admin MFA policy; never retrieve session secrets |
| Functions | `community-api` entry `src/main.js`; `community-maintenance` entry `src/cleanup.js`, Node 22; runtime row read/write | List/get functions and active deployment metadata, execute roles, events, schedule, runtime scopes, logging settings; do not run functions or expose environment secrets |
| Retention | Cleanup removes only quotas more than 1 day past expiry, max 2000/run; setup says hourly; admin cleanup 100/call | Actual deployed source/version, schedule, execution-log/provider retention and backup settings |
| Domains/platforms | Admin browser and native daemon mentioned in setup/admin docs | List/get exact IDs, hostnames, platform types, callbacks/TLS; no guessed allowlist |

For schema discovery, retrieve attribute/column and index metadata, not user
documents. Do not enumerate users or execution request/response bodies just to
check configuration. Record unavailable settings as unknown rather than
substituting historical docs. If an auth/DPA/retention setting is not exposed by
read-only MCP, obtain owner console evidence in a later task.

The approval plan is [APPWRITE_CHANGES.md](APPWRITE_CHANGES.md). Policy constants
are in `appwrite/community/src/retention.js`; only quota grace currently affects
runtime cleanup. No remote alignment changes have been applied.
