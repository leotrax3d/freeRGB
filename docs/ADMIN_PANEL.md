# FreeRGB admin and community operations

## Hosting and login

The admin application is a static JavaScript/Vite app at
`https://leotrax3d.github.io/freeRGB/admin/`. It is assembled alongside the existing
website and isolated desktop demo by `scripts/build-website.mjs`. GitHub Pages is
public hosting; no inbox data, service keys or session credentials are built into
the site. Appwrite checks authentication and admin-team membership on every admin
request. Visiting the login page alone grants no access to submissions.

The first admin account is `freergb-admin`, email `freergbteam@gmail.com`, with
confirmed membership in `community-admins`. The account has no generated password;
use the one-time email code requested in the login form. Codes are sent only when
the user requests them. Additional administrators must be explicitly added to this
team by an owner. The Appwrite Web app permits exactly `leotrax3d.github.io`.

`apps/admin/appwrite.js` owns the Web SDK configuration and performs one `ping()`
per page load. Desktop networking remains in the daemon; its anonymous identity
is separate from the admin account. No administrator credentials are stored there.

## Available operations

- Inbox pages of 25 submissions, filtered by New/In progress/Resolved/Spam.
- Original feedback/hardware/error payload and conversation history.
- Developer replies visible only to the submitting anonymous identity.
- Internal notes hidden from the app user, including in thread API responses.
- Block/unblock an identity for new writes; existing private messages stay readable.
- Explicit deletion confirmation removes the report and its conversation.
- An append-only audit table records actor/action/target, without report text.
- Real totals for submissions, votes, downloads, blocked identities and status;
  per-effect public averages and download totals for the published catalog.

The desktop Settings page contains **Your messages → Check replies** and a
conversation view. Follow-ups share the submission quota. Losing/deleting the
anonymous session loses access to its conversations; there is no email recovery
for anonymous identities. Reports do not automatically expose an email address.

## Active protection

Appwrite Firewall has two enabled rules (the Free plan's rule allowance):

| Rule | Scope | Limit |
|---|---|---|
| `community-executions-ip` | POST `/v1/functions/community-api/executions*` | 60/min/IP |
| `anonymous-sessions-ip` | POST `/v1/account/sessions/anonymous` | 10/hour/IP |

Persistent private quota slots enforce 10 vote writes/minute/identity,
5 submissions/follow-ups/hour/identity, 20 successful version receipts/hour and
30 admin messages/minute. Slots claim deterministic row IDs; concurrent Function
workers cannot bypass the quota by losing a counter update. Duplicate report and
message request IDs reuse the accepted operation rather than delivering twice.

`community-maintenance` has no client execute permissions and runs at a 60-minute
interval to remove expired quota slots older than a day. This uses the supported
Free-plan interval, not a paid cron schedule. It never removes user reports.

Only IDs/versions in the bundled published catalog can receive download receipts;
unknown IDs are rejected. Summary results are cached for up to 30 seconds per
worker, invalidated locally after votes/downloads. Other workers can show data up
to 30 seconds old. The existing daemon refresh cache can add up to a minute.

The configured API rules protect the execution endpoint the applications use.
A separately enabled public Function domain needs an additional edge rule;
the current Free plan has no spare rule. Do not add an alternate unprotected
public endpoint. Standard Cloud DDoS protection is still provider-managed.
Anonymous accounts, IP limits and client install receipts cannot prove one human
per identity or cryptographically prove a download. Adaptive challenges, strong
download tickets and provider cost-alert integration remain future hardening;
they are not falsely represented as enabled.

## Schema

Database `community`: existing `ratings`, `downloads`, `submissions` (with status),
plus `messages`, `audit`, `quota_slots`, `blocked_users`. New tables grant no
client permissions. The Function uses only `rows.read`/`rows.write`; JWT-backed
`GET /account` and `GET /teams/community-admins` determine the actual actor and
membership without the privileged runtime key.

Public users can call `inbox`, `thread` and `reply` only for their own identity.
Administrative calls are `admin.list`, `admin.thread`, `admin.stats`,
`admin.reply`, `admin.note`, `admin.status`, `admin.block`, `admin.unblock`,
`admin.delete`, `admin.audit`, `admin.cleanup`. Submission IDs and cursors are
validated before use in paths. The public GUI cannot invoke administrative
actions through its daemon allowlist.

## Validation and deployment

`npm test` in `appwrite/community` checks isolation, notes, quotas (including
concurrent claims), blocking, duplicate messages, deletion and traversal rejection.
The opt-in `live-admin-test.mjs` takes an ephemeral administrator JWT through
hidden stdin; it uses synthetic records, checks the real WAF rule header and cleans
the test submissions. Operator cleanup removes only its known test users,
quota rows and audit events, and revokes the temporary admin test session.

Upload `package.json`, `catalog.json` and `src/` as a tar.gz. Use `src/main.js`
for `community-api`, `src/cleanup.js` for `community-maintenance`, Node 22 and an
empty build command. Neither function deployment publishes the 25 local effects.

The Pages workflow may deploy from `main` or `codex/appwrite-community`; both are
explicitly permitted in the `github-pages` environment. This allows testing the
requested branch without merging it into main. Website/demo/admin share one
artifact and deployment concurrency group, so the admin does not replace the
existing website with an admin-only artifact.

Further useful additions: assignments, tags/priority, search, attachment review,
retention settings, MFA enrollment, notification preferences, moderation queues,
version-release approvals and operational alerting. None should grant anonymous
users broader read access or introduce secrets into the public Pages bundle.
