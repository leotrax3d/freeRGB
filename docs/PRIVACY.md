# freeRGB privacy notice - DRAFT FOR LAWYER REVIEW

> **Status: engineering draft. Not reviewed or approved by a lawyer. Do not publish
> as is.** It describes what the code does on the repository state this file was
> written against (task FR-006 in `docs/legal/RISK_REGISTER.md`). Every item in
> square brackets is an open owner or lawyer decision (`docs/legal/OWNER_DECISIONS.md`,
> D4 and D8). The live Appwrite project was **not** inspected for this draft;
> statements about server behaviour come from the function source in
> `appwrite/community/` and from `docs/APPWRITE_SETUP.md` / `docs/ADMIN_PANEL.md`.

## 1. Controller

[Name, address and contact of the controller for the community service, the
website and the download pages. Not yet decided (D1, D4). Contact for privacy
requests: [e-mail]. Data protection officer: [required? to be decided].]

## 2. Summary

- freeRGB controls lighting and cooling hardware **locally**. No account is
  needed, and lighting, cooling and profiles never leave the device.
- The program **does** contact external services in the cases listed in the
  data-flow table (section 3): GitHub (effect catalog and downloads, module
  downloads, update check) and, **only if you switch it on**, the community
  service hosted on Appwrite Cloud.
- The online community features (ratings, report inbox, report upload) are **off
  by default**. Counting of your effect downloads is a **separate opt-in**,
  also **off by default**, and works only while the community features are on.
- Earlier text that said "no cloud or telemetry" was inaccurate and has been
  replaced. freeRGB does not collect usage analytics or crash reports
  automatically; it still makes the network requests listed below.

## 3. Data-flow table

"Default" is the behaviour of a fresh install of this branch.

| # | Flow | Trigger | Data sent | Recipient | Default | Code |
|---|---|---|---|---|---|---|
| F1 | Effect catalog and package download | Marketplace "check for updates" and "install" (user action) | HTTPS request to GitHub (`leotrax3d/FreeRGB`, branch `effects`); IP address and request metadata visible to GitHub; no freeRGB identifier | GitHub | active when used | `marketplace.rs` `Manager::refresh/install`, `freergb-modules/src/source.rs` |
| F2 | Hardware module catalog and download | module install/update | same as F1 (GitHub releases API) | GitHub | active when used | `freergb-modules/src/source.rs` |
| F3 | Application update check | update page of the GUI (a setting can run it automatically, see the update docs) | GitHub API request with `User-Agent: freeRGB/<version>`; IP address | GitHub | per update settings | `freergb-update/src/lib.rs` |
| F4 | Rating summaries | Marketplace page opened or "refresh ratings" | list of effect IDs (max. 64 per request); IP address and request metadata; **no identity** | Appwrite (function `community-api`) | **off** | `ratings.rs` `fetch`, `main.js` `summary` |
| F5 | Anonymous session creation | first time a vote, report or counted download needs an identity | empty request to Appwrite; Appwrite creates an anonymous user and returns a session cookie, stored on the device in `appwrite-session-<hash>.json` in the `effect-marketplace` data folder (a credential file) | Appwrite | **off** | `ratings.rs` `jwt` |
| F6 | Vote | user clicks stars | effect ID, stars, Appwrite user ID (server derives it from the verified token) | Appwrite database `ratings` | **off** | `ratings.rs` `rate`, `main.js` `rate` |
| F7 | Download receipt (count) | after a successful, verified install of a non-bundled effect | effect ID, version, Appwrite user ID; receipt is queued on device (`appwrite-session-<hash>.downloads/`) until sent | Appwrite database `downloads` | **off (separate opt-in)** | `marketplace.rs` `install`, `ratings.rs` `download`, `main.js` `download` |
| F8 | Own votes lookup | after identity exists | effect IDs, token | Appwrite | **off** | `main.js` `mine` |
| F9 | Report upload | user opens the report dialog, reviews the editable JSON and confirms | the reviewed report JSON (kind `feedback`, `hardware` or `error`, max. 30 KB; hardware reports use the redacted diagnostics: application version, OS, architecture, runtime counters, USB VID/PID, LED counts and similar), request ID, Appwrite user ID | Appwrite database `submissions` | **off**, always explicit | `ratings.rs` `submit`, `report.rs` |
| F10 | Report inbox and replies | user opens the inbox or replies | token, reply text (max. 6000 chars); developer replies are stored as messages | Appwrite database `messages` | **off** | `ratings.rs` `inbox`, `admin.js` |
| F11 | Moderation | operator action | block flag keyed by user ID, audit entry (operator ID, action, target) | Appwrite `blocked_users`, `audit` | operator only | `admin.js` |
| F12 | Abuse quota | every authenticated community call | hashed slot ID derived from user ID, action and time window, plus expiry | Appwrite `quota_slots` | **off** | `security.js` |
| F13 | Website: fonts | visitor opens the website | request to `fonts.googleapis.com` (stylesheet `@import`), so the visitor's IP address goes to Google | Google | active | `website/src/index.css` |
| F14 | Website: releases | visitor opens the download section | browser request to `api.github.com/repos/leotrax3d/FreeRGB/releases` | GitHub | active | `website/src/App.tsx` |
| F15 | Provider-side logs | any request to Appwrite or GitHub | IP address, timestamps, user agent, request metadata held by the provider under its own policy; Appwrite also keeps session metadata for anonymous sessions and function execution logs [content and retention to be verified against Appwrite documentation and the DPA] | Appwrite, GitHub, Google | n/a | provider |

Notes:

- Development builds started with an effect development directory never create
  a community client and send nothing (`Manager::development`).
- Bundled example effects (aurora, comet, prism) are never counted.
- The GUI never sees tokens or Appwrite user IDs; the daemon holds them.
- No analytics, advertising or crash-report service is built in. Hardware
  inventory, sensor values, profiles, macros and screen or audio content are not
  sent anywhere by the features described above.
- A random identifier plus an effect/vote/download history is personal data
  (pseudonymous), not anonymous data, because it can single out a device's
  history. The notice treats it as personal data.

## 4. Purposes and legal bases

Legal bases are **placeholders for the owner and a lawyer**. They are not decided
and no basis is claimed here.

| Purpose | Data | Legal basis |
|---|---|---|
| Show effect ratings and download totals | F4 | [Art. 6(1)(a) or (f) GDPR - to be decided] |
| Let a user rate effects | F5, F6, F8 | [to be decided] (user requests the feature) |
| Count downloads for popularity statistics | F5, F7 | [to be decided]; proposal: consent only (opt-in), as implemented |
| Receive, answer and moderate reports | F9-F11 | [to be decided] (user requests contact) |
| Prevent abuse and overload | F12, F15 | [Art. 6(1)(f) - legitimate interest, balancing to be written] |
| Serve the website | F13-F15 | [to be decided; the Google Fonts request in particular] |
| Deliver effects, modules and updates | F1-F3 | [to be decided] (GitHub as host) |

Storing the session file on the device is a storage operation on terminal
equipment. Whether and how section 25 TDDDG applies to each storage operation
must be assessed by a lawyer (see FR-006 "Maßnahme"); this draft does not assert a
blanket consent requirement.

## 5. Recipients and processors

- **Appwrite** (Appwrite Cloud, project in region `fra`, endpoint
  `fra.cloud.appwrite.io`) hosts the community database and function.
  [Processing agreement (Art. 28), data location, sub-processors and any
  third-country transfer to be checked and documented.]
- **GitHub** hosts the repository, effect catalog, module and application
  releases. [Role and transfer basis to be assessed.]
- **Google Fonts** (website only). [To be decided; self-hosting the font removes
  this flow.]
- The operator (and members of the `community-admins` team) can read reports,
  messages and, via the Appwrite console, raw ratings and download rows.

## 6. Retention and deletion

Technical state as implemented (details in `docs/APPWRITE_SETUP.md`):

| Data | Server retention today |
|---|---|
| `quota_slots` | deleted by the scheduled function `community-maintenance` (60-minute interval) when expired for more than 24 hours; the same operation exists as admin action `admin.cleanup` (max. 100 rows per call) |
| `ratings` | kept until the user withdraws the vote in the app (`stars: null` deletes the row); no automatic expiry |
| `downloads` | **no deletion and no expiry** |
| `submissions`, `messages` | kept until an operator deletes a submission (`admin.delete` removes the submission and its messages); no automatic expiry |
| `audit` | **no expiry** |
| `blocked_users` | until an operator unblocks |
| Appwrite anonymous users and sessions | **not deleted** by any freeRGB function |
| Provider logs | [per provider; not verified] |

Local state (device): the session file and queued receipts live in the
`effect-marketplace` data folder. Turning counting or the community features off
in the Marketplace privacy settings deletes unsent receipts. The session file is
kept until the user deletes it [uninstaller behaviour is not verified]. Turning
the community features off does not delete server-side data.

**Missing before any online operation (open work, no decision taken):**

1. A documented retention period per table [owner/lawyer], with a scheduled
   function that enforces it (today only `quota_slots` is cleaned).
2. A way to delete a person's server-side data: ratings, download rows, reports,
   messages, blocked entry **and** the Appwrite anonymous user. No such
   operation exists, neither self-service nor as an operator runbook.
3. A way to *identify* a requester: the GUI deliberately hides the Appwrite user
   ID, so a person cannot name the record that belongs to them. A "show my
   anonymous ID / delete my data" action is needed (or a documented alternative).
4. A data-export (access) procedure.
5. Verified Appwrite log, session and execution-log retention settings.
6. Controller, legal bases, processing agreement and the final Art. 13 text.

## 7. Your rights

[To be completed by the lawyer: access (Art. 15), rectification (16), erasure
(17), restriction (18), portability (20), objection (21), withdrawal of consent
at any time (7(3)), complaint to a supervisory authority (77). Contact: [e-mail].
State how the pseudonymous record is linked to a requester, see section 6 item 3.]

You can stop all community processing at any time: Marketplace page, "Online
community and privacy", untick the features. Local effect installs keep working.

## 8. Settings (implemented)

The switches live in the daemon (`effect-marketplace/community.json`) and are shown
on the Marketplace page. Both are `false` unless the user changes them.

| Setting | Meaning |
|---|---|
| `online` | Allows contact with the community service (F4-F12). Off: no request, no identity file, no report upload, ratings hidden. |
| `count_downloads` | Allows F7. Only effective while `online` is on. Off: no receipt is queued or sent and no download record or identity is created by installing an effect. |

Daemon method: `marketplace.community.set` with `{online?: bool, count_downloads?: bool}`.
An unreadable settings file is treated as "off".

Effects still install from the official catalog (F1) with the community features
off; that flow involves GitHub only and no freeRGB identity.

Operators can also disable the client completely at build or run time by setting
`FREERGB_APPWRITE_ENDPOINT` and `FREERGB_APPWRITE_PROJECT_ID` to empty values.

## 9. Changes

[Version and date of this notice; how users are told about changes.]
