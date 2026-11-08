# D4 Appwrite findings and proposed changes

Date: 2026-11-08. Scope: this worktree, `docs/` and `appwrite/` only.
No remote create, update, delete, deployment, execution, or data deletion occurred.
This is an engineering proposal, not legal approval.

## Inspection result

The connected Appwrite MCP `appwrite_get_context` returned `UNAUTHORIZED`,
`oauth_token_invalid_grant`, reauthentication required. The remote inspection
could not proceed. No live region, schema, permissions, auth settings, functions,
schedule, retention, platforms or domains were verified. No user records or
credentials were retrieved. Do not treat historical setup notes as live evidence.
The owner can reconnect the Appwrite integration for a later list/get-only audit.

Repository evidence: `docs/APPWRITE_SETUP.md`, `docs/ADMIN_PANEL.md`,
`appwrite/community/src/{main,admin,security,cleanup}.js`. The recorded project is
`6adb5259001299138e7b`, endpoint `https://fra.cloud.appwrite.io/v1`, database
`community` (TablesDB). `fra` suggests Frankfurt/EU; actual region is unverified.
See [APPWRITE_ALIGNMENT.md](APPWRITE_ALIGNMENT.md) for the full gap inventory.

## Owner decisions recorded

The private owner is the controller; personal name, service address and privacy
contact still need to be supplied. Consent under Art. 6(1)(a) GDPR is the selected
basis for accounts and ratings. Community access and download counting default
off; counting requires a separate opt-in. EU hosting is required. DPA acceptance
is owner-reported, not independently verified; retain the accepted version and
acceptance evidence. These decisions do not establish legal approval or prove
that the live service enforces consent, withdrawal or erasure.

## Exact proposed remote changes for separate owner approval

Each number is a proposed operation or bounded group of operations, not authority
to execute it now. Resolve live inventory first using list/get only. If the live
state differs, revise the affected proposal before approval. No bulk data purge
is authorized by this document.

1. **Region and processor record:** retain project `6adb5259001299138e7b` only if
   its verified data region is EU (`fra` preferred). If it is outside the EU,
   pause online release and prepare a separate migration plan; do not delete or
   migrate the current project under this proposal. Record the DPA version,
   acceptance date, covered services, sub-processors and transfer safeguards in
   the owner's private operational record. Do not publish private contract data.
2. **Database access:** for the `community` tables `ratings`, `downloads`,
   `messages`, `quota_slots`, `audit`, `blocked_users`, set table permissions to
   `[]` and row security to `false`. For `submissions`, set permissions to only
   `read("team:community-admins")` and row security to `false`, preserving the
   documented admin-console workflow. Grant no public/client create, update or
   delete rights. Verify table/collection inventory and existing permissions
   first; do not remove unknown resources or user data. Keep runtime credentials
   server-side with only `rows.read` and `rows.write` for current functions.
3. **Cleanup deployment and schedule:** after review and offline tests, deploy
   the quota-only `src/cleanup.js` and its `src/retention.js` dependency to
   `community-maintenance`, Node 22, entry `src/cleanup.js`, empty build command,
   execute permissions `[]`, no event triggers, hourly interval (60 minutes).
   Keep at most 20 batches of 100 per run and delete only quota rows whose
   `expires` is strictly older than now minus 1 day. Verify the provider's actual
   interval setting before deployment. A deployment is a separate write action.
4. **Extended row retention (implementation required before deployment):** use
   `ratings.$updatedAt` for 180 days after last vote; `downloads.$createdAt` for
   7 days; `submissions.$createdAt` for 30 days and cascade all linked messages,
   including internal notes and newer replies; `audit.$createdAt` for 30 days;
   `blocked_users.$createdAt` for 30 days. Use strict older-than cutoffs, UTC,
   hourly runs and bounded pagination with backlog monitoring. Add indexes only
   where the verified schema/query plan requires them; do not guess attribute
   types or rename/drop attributes. Confirm backups/recovery and approve a
   count-only preview of affected rows before enabling any new deletion rule.
   Current source does not implement these deletions. Receipt deduplication and
   popularity totals become limited to the retained 7-day window; rating totals
   become limited to retained votes. Accept these effects rather than keeping
   indefinite user-linked history or archives. Blocks expire after 30 days.
5. **Accounts, sessions and withdrawal (implementation required):** set session
   maximum lifetime to 30 days using the verified auth setting. Keep anonymous
   auth for the existing consent-based client; leave other providers unchanged
   until their actual use is known. Delete inactive anonymous accounts after
   180 days from last authenticated activity (fall back to account creation if
   never active), with a reliable server-side activity record. First erase their
   ratings, receipts, submissions/messages, blocks and attributable audit data,
   then sessions and account; never delete console/operator identities. Implement
   authenticated export, immediate consent withdrawal/erasure and session
   revocation before enabling this job. Add only the dedicated job's verified
   minimum user/session scopes; never broaden `community-api` credentials for it.
6. **Logs and backups:** request function execution-log retention of 7 days,
   provider request/security logs of 7 days and backup expiry of 30 days. Disable
   request-body/JWT/cookie logging. Verify available controls and contractual
   limits; if these targets cannot be set, record actual limits and revisit the
   notice and owner approval before release. Restore procedures must reapply
   erasure records before serving restored data. No log or backup deletion now.
7. **Functions and consent:** retain public execution only for `community-api`
   as required by its summary/JWT contract, with JWT verification for identity
   actions; maintenance stays non-public. Deploy future consent/withdrawal
   enforcement only after implementation and tests: no account/rating activity
   without consent; no receipt collection without separate counting consent.
   Default counting stays off. Do not invoke functions as part of this audit:
   even an execution used for testing can write data.
8. **Platforms and domains:** after list/get inventory, retain only documented
   production desktop platform(s), the actual admin website hostname and required
   callback/domain entries. Remove development/unused entries only by a separate
   approval naming their exact IDs and hostnames. No such removal is currently
   actionable because MCP could not return the inventory; do not infer domains
   from examples or add wildcard hosts. Review TLS/callback settings at that time.

## Repo implementation and limits

`src/retention.js` records all proposed day limits and hourly cleanup bounds.
Only the existing 1-day quota grace is wired into both scheduled cleanup and
`admin.cleanup`; its behaviour is unchanged. Other limits are not enforced by
adding this file. Nothing was deployed. `PRIVACY.md` and `APPWRITE_SETUP.md`
distinguish source behaviour, owner decisions, proposals and unverified live state.
Live test scripts were deliberately not run because they create remote data.

Validation: `npm.cmd test` in `appwrite/community` passed both the existing API
test and the new mocked cleanup test. The latter verifies the 1-day quota
cutoff, quota-only access and deletion of the returned expired slot without
contacting Appwrite. `git diff --check` passed. Plain `npm test` was blocked by
the local PowerShell script policy; `npm.cmd` required no policy change.
