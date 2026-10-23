# FreeRGB Appwrite community service

Configured and live-tested on 2026-10-23. The daemon now uses Appwrite. The old
Supabase migration is historical source only.

| Resource | Public value |
|---|---|
| API endpoint | `https://fra.cloud.appwrite.io/v1` |
| Project | `6adb5259001299138e7b` |
| TablesDB database | `community` |
| Ratings table | `ratings` |
| Download receipts table | `downloads` |
| Feedback/hardware/error reports table | `submissions` |
| Function | `community-api` |
| Active deployment | `6adb736d938eab9ef468` |
| Admin team | `community-admins` |

No API key belongs in the desktop application. The Function gets a temporary
runtime key with only `rows.read` and `rows.write`. Anonymous auth is enabled.
The daemon persists its session cookie in its state directory, namespaced by
endpoint and project. It gets a fresh JWT per authenticated operation. Rejected
sessions produce an error rather than silently creating a new voting identity.
Session files are credentials: never export, log or commit them.

Public defaults are built in. Override with `FREERGB_APPWRITE_ENDPOINT` and
`FREERGB_APPWRITE_PROJECT_ID`; set both empty to disable. Distributed builds
require HTTPS. The GUI never receives tokens or connects directly to Appwrite.

## Access

All tables disable row security. Ratings/downloads grant no client permissions.
Submissions grant only `read("team:community-admins")`. Only the Function and
project administrators can access raw ratings/downloads; users cannot read or
modify anyone's submissions. Project admins can inspect reports in the Appwrite
console now. The optional admin team currently has no project users; add trusted
users manually for a future admin client. Console users and project users differ.

The Function verifies JWTs at `/account`, without its runtime key. Record IDs
derive from that verified user, never a caller-supplied user ID. Public summaries
contain only average, vote count and download count. No report-list endpoint exists.

## Contract

Invoke `POST /functions/community-api/executions` with `async:false`, a string
`body` containing a payload below, and for authenticated actions an execution
header `x-freergb-jwt` from `POST /account/jwts`. First create and persist the
native cookie from `POST /account/sessions/anonymous`. Check the execution's
`responseStatusCode` as well as the outer HTTP result.

```json
{"action":"summary","effects":["aurora"]}
{"action":"mine","effects":["aurora"]}
{"action":"rate","effect_id":"aurora","stars":5}
{"action":"rate","effect_id":"aurora","stars":null}
{"action":"download","effect_id":"aurora","version":"1.0.0","success":true}
{"action":"submit","kind":"feedback","request_id":"unique-request-id","reviewed":true,"payload":{"message":"Example"}}
```

One vote per anonymous identity/effect. A download counts once per identity,
effect and version; retries, reinstalls and double clicks do not increase it.
A new version counts again. The daemon queues a receipt after hash/WASM
validation and installation. Bundled examples and failed installs are excluded.
Offline receipts persist and retry on Marketplace refresh. Anonymous identity
prevents accidental duplicates, not deliberate Sybil abuse; a client receipt
cannot independently prove that a malicious client downloaded a file.

Summary calls accept 64 effect IDs per batch. Votes are paginated; a materialized
summary would reduce latency/cost at larger scale. Feedback/hardware uploads
require an editable preview and explicit Send. The exact reviewed JSON is sent.
Hardware uses the existing redacted diagnostics, including optional board/chip
support data. No automatic hardware submission is added. Request IDs deduplicate
submission retries too.

## Checks and deployment

Function source: `appwrite/community/src/main.js`. Node 22, ESM, no dependencies,
empty build command, entry `src/main.js`. Upload a tar.gz of `package.json` and
`src/`, then activate it. Run `npm test` in `appwrite/community` for API tests.
The opt-in `node appwrite/community/test-live.mjs` uses synthetic data, prints
disposable IDs for cleanup and never prints credentials. The initial live test
passed; its synthetic records and users were then removed.

No installer/release has been published. Existing installations use these
features only after updating to a build containing the changes.
