# Official effect marketplace

The modern desktop app has a Marketplace view for discovering, installing,
updating, configuring, activating and uninstalling official effects. The daemon
owns downloads, installed packages, configuration and rendering. Closing the GUI
has no effect on lighting. The existing daemon service/autostart restores the
selected effect after login/reboot; this feature does not install a second service.

## Architecture

The host, package manager and examples are Rust. Built-in effects keep their
existing native render path. Downloadable effects are portable `no_std` Rust
compiled to `wasm32-unknown-unknown`, executed by `wasmi` inside the daemon.
There is no native dynamic-library loading or JavaScript execution.

One guest instance is reused for the active selection. Parameters are sent once
on activation; each frame crosses the host/guest boundary once and returns a
contiguous RGB buffer. A shared surface of at most 128×64 is sampled into the
existing 320×200 canvas, then existing device workers handle placement,
brightness and output. Runtime cost does not grow with the number of devices.
This resolution is appropriate for the current LED layouts; it is an explicit
API-v1 quality limit, not a promise of arbitrary screen-resolution rendering.

A local release-mode benchmark (`cargo run --release -p freergb-effect-host
--example benchmark`) measured approximately 0.94–1.56 ms per frame for the
three examples, including sampling into the 320×200 canvas, over 300 frames each
in the onboarding cloud instance. This is a sample, not a cross-machine latency
guarantee or a benchmark of audio capture or physical device I/O.

Downloads run on separate threads without holding the engine mutex. Only one
catalog/install job runs at a time. Activation validates the installed reference
and parameters and saves the new selection before acknowledging success.
Profiles, macros, the lighting inspector and Classic can carry the serializable
selection; editing downloaded parameters happens in the modern Lighting tab.
Marketplace effects cannot be nested inside the existing custom-layer format in
API v1, avoiding ambiguous guest budgets and package dependencies.

## Trust and failure boundaries

Production source is fixed to `github:leotrax3d/FreeRGB@effects`. The existing
GitHub release-asset transport is reused. Catalogs and packages come from that
repository over verified HTTPS. Catalog entries may name release assets only;
arbitrary URLs, paths and repositories are rejected. SHA-256 and exact size bind
each downloaded program to its entry. This is a GitHub/TLS trust model, **not an
Ed25519-signed catalog**. The hardware-module signing keys are deliberately not
repurposed, and their signature requirements remain unchanged.

API v1 requires `permissions: []` and rejects unknown permissions and API
versions. Guests have no WASI or other imports, so there is no direct filesystem,
network, device, audio, screen, system-information or input access. Future host
APIs must add explicitly versioned, permission-checked snapshots of daemon-owned
inputs rather than exposing OS APIs to guests. A future API can extend this
boundary for audio features, screen samples, sensors and events; these inputs
are not implemented for downloadable effects yet. Existing native audio/screen
effects continue to work as before.

Limits: 512 KiB per program, 8 MiB guest linear memory, one memory/table, 1,024
table entries, 16 finite numeric parameters, 256 KiB/128 entries per catalog and
2,000,000 interpreter fuel units per frame. Start functions and imports are
rejected; initialization shares a bounded fuel allowance. Traps, exhausted fuel
or invalid output buffers fault that effect and render black. The daemon and
API remain available, and the fault is shown in Marketplace. Reapplying settings
or choosing another effect resets the fault. A malformed installed package at
startup falls back to the native default with a visible error; it does not stop
the daemon. Failure to initialize the marketplace store disables Marketplace
while allowing built-in lighting to start.

Package versions are immutable. Updates install alongside older versions;
active selections remain pinned to their version until explicitly activated.
The catalog sequence cannot decrease or change contents at the same sequence.
Catalog, metadata and installed-index writes use temporary files and rename;
package contents and hashes are rechecked on activation and restart. Offline
installed effects need neither a catalog refresh nor network access.

Uninstall removes the installed registration and prevents activation. It is
rejected while that package is active. Immutable old versions and verified
downloads are retained as a local cache, so an older saved profile can work after
reinstallation. A missing installation produces a clear error rather than
silently substituting another effect. This cache does not allow uninstalled
programs to run.

## Local development collection

The additional 25 effects use the existing startup-only directory source. See
[EFFECT_COLLECTION.md](EFFECT_COLLECTION.md) for the complete list, controls,
local build/run instructions and limits. Directory-source managers disable the
community client, so local installs and ratings cannot change live Appwrite data.
The official bundled catalog and publishing workflow retain the original examples.

## Shipped examples and publication

`effects/catalog.json` and `effects/packages/` contain three real compiled Rust
effects: **Aurora**, **Comet**, and **Prism**, with speed and intensity controls.
They are bundled so the first install works offline before an official effects
release exists. They still pass the same hash, schema, sandbox and persistence
checks as downloaded packages; they are not GUI simulations. Previews are static
colour suggestions, clearly labelled in the GUI.

Rebuild the bundled examples from the repository root:

```bash
rustup target add wasm32-unknown-unknown
python3 scripts/build-effects.py
cargo test -p freergb-effect-host --locked
```

For a new release, use a new version and strictly increasing sequence:

```bash
python3 scripts/build-effects.py --version 1.0.1 --sequence 2 --out-dir /tmp/effect-release
```

The **Official effects** GitHub Actions workflow builds portable programs and
uploads reviewable workflow artifacts by default. With its explicit `publish`
input, it uploads packages first and the catalog last to the official rolling
`effects` release. Old assets are retained. The first release is created as a
**draft**: a maintainer must review and publish it before public clients can
fetch it. Subsequent runs reject sequence rollback and reused version names.
The workflow must be merged into the official repository before it can be run;
no GitHub release was published as part of this implementation task.

Live refresh needs `api.github.com` and GitHub release-asset destinations.
Private repositories use the existing optional `FREERGB_GITHUB_TOKEN` transport
binding; do not put credentials in catalogs or packages. API access is distinct
from authenticated Git checkout access. In the onboarding instance the live
API request was blocked by egress policy. `api.github.com` was added to the
cloud environment configuration draft; it must be applied in environment
settings before retrying. Live GitHub download/publication is not claimed as
validated. The same fetch/size/hash/install path was exercised using a local
release fixture.

For explicit offline development, the daemon accepts startup-only
`--effect-dev-directory /absolute/release`. Put `catalog.json` and asset files
at the root of that directory. There is no GUI or RPC source override. On Linux,
installed packages are under `$XDG_DATA_HOME/freergb/effect-marketplace`; on
Windows under `%LOCALAPPDATA%/freeRGB/effect-marketplace`. The active reference
and parameter values use the existing `state.json` and profile paths.
`--no-state` uses a disposable store and does not promise restart persistence.

## Guest ABI v1

The module exports:

- `memory`: its bounded linear memory.
- `set_param(index: i32, value: f64)`: parameter order follows the catalog schema.
- `render(time_seconds: f64, width: i32, height: i32) -> i32`: offset of
  `width * height * 3` tightly packed, row-major RGB8 bytes in exported memory.

The buffer belongs to the guest and remains valid until the next render. Negative
or out-of-bounds pointers fault the effect. Resolution never exceeds 128×64.
Time is elapsed since activation; no wall-clock/system data is exposed.
`effects/examples/effect.rs` demonstrates static buffers without allocations.
Additional authors may use any language targeting this ABI; production code
accepts only artifacts named by the official catalog. See the example catalog
for metadata, supported numeric parameter schemas and preview colours.

## Local API and verification

| Method | Parameters | Result |
|---|---|---|
| `marketplace.status` | none | `packages: {source, catalog, installed, job, error}`, `active`, `runtime_error` |
| `marketplace.refresh` | none | `true` when job accepted; poll status until `job: null` and inspect `error` |
| `marketplace.install` | `{id}` | `true` when install/update accepted; poll status for progress/completion |
| `marketplace.uninstall` | `{id}` | `true`; rejects active packages or concurrent jobs |
| `effects.set` | `{type:"marketplace", id, version, parameters:{speed:1, brightness:0.8}}` | `true` only after loading and saving the selection |

Validation failures return normal local-API errors. Jobs expose action, id,
stage, bytes and total; failed jobs retain their error and previous installation.
A successful RPC accepting a job does not mean the package is already installed.

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cd apps/desktop && npm test && npm run build
# From repository root, after building freergbd:
python3 scripts/test-effect-marketplace.py
```

The process smoke uses real Linux daemon processes, private temporary user
state/socket directories, the actual file-download transport, JSON-RPC,
installation, rendering, version-pinned updates, graceful process restart,
offline restoration, uninstall and corrupted-package recovery. It never opens
hardware or changes personal configuration. Rust integration tests also cover
the cross-platform engine/API path. Host tests exercise fuel exhaustion,
invalid pointers, traps, forbidden imports, memory limits, tampering, metadata,
parameter validation, failed installation and catalog rollback. UI tests cover
catalog defaults/filtering and browser-demo refusal to install. A headless-browser
interaction check through a temporary test-only browser-to-socket transport also
verified discovery, install, parameter editing, activation, search, responsive
layout, the lighting inspector and continued rendering after the UI closed. This
bridge changes no product transport code and does not validate the native window. Interactive
native-window/hardware acceptance and live GitHub publication remain separate.

## Publishers, shared ratings and reviewed error reports

Catalog entries now include `publisher: {id, name}`. Legacy official entries
without that field default to the freeRGB Team; unsafe/invalid publisher metadata
is rejected. Publisher names are searchable and shown on the effect cards.

Shared 1–5 star ratings use Appwrite anonymous sign-in, with no user account UI
or GitHub voting step. The Rust daemon owns session tokens, network operations
and persistence. Ratings are optional at application startup; without a configured
project the UI clearly shows that the shared service is not connected. See
[the setup guide](APPWRITE_SETUP.md) for the resources, public configuration,
privacy, abuse limits and deployment prerequisite.

Public summaries include successful download counts. Marketplace sorting supports
downloads, average rating and name. The Settings developer-contact section offers
feedback and optional hardware reports with an editable preview before sending.
`community.submit` accepts `{kind, payload, request_id, reviewed:true}`; submissions
are readable only by project admins and the `community-admins` team.

`marketplace.ratings.refresh` starts a cached background refresh (at most once
per minute). `marketplace.rate` accepts `{id, stars: 1..5}` or `{id, stars: null}`
for withdrawal; server acceptance is observed before updating the local rating.
`marketplace.status.packages.ratings` contains `configured`, `by_effect`, `mine`,
`updated_at`, `loading`, `saving` and `error`. Neither tokens nor anonymous user
ids are exposed to the GUI. A failed aggregate refresh does not erase a vote
that the server has already accepted; the UI keeps that accepted vote and warns
that totals could not be refreshed.

Marketplace operation errors expose `FX_CATALOG_FAILED` or `FX_INSTALL_FAILED`;
activation/render errors expose `FX_ACTIVATION_FAILED` or `FX_RUNTIME_FAULT`.
`marketplace.report.prepare` with `{kind:"operation"|"runtime"}` requires a
current corresponding failure and returns a reviewable title/body/payload. It
never submits anything. The native desktop opens only allowlisted contact URLs
or a length-bounded, percent-encoded issue composer in the official repository.
