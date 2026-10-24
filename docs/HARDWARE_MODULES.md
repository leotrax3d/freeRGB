# Hardware modules and device setup

freeRGB detects attached lighting hardware, and after the user selects it,
downloads only the modules those devices need. Installed modules control the
hardware directly from then on and work offline. The first module ports the
Razer driver from [OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB).

## Overview

```
GUI (Tauri)  --JSON-RPC-->  freergbd  --stdio, module protocol v1-->  module process
 setup wizard                ├ setup manager (catalog, matching, jobs)     (one per device)
 new-devices panel           ├ store (verified install, rollback)
                             ├ hardware scan + HID gate  <── owns every device handle
                             └ engine (frames, light test)
```

| Piece | Where | License |
|---|---|---|
| Module protocol + module SDK | `crates/freergb-module-api` | MIT OR Apache-2.0 |
| Catalog, trust, packages, store, downloads, setup, process host | `crates/freergb-modules` | MIT OR Apache-2.0 |
| Release tool (keys, pack, source archive, catalog, sign, verify) | `crates/freergb-module-tool` | MIT OR Apache-2.0 |
| Daemon glue + `modules.*` API | `crates/freergb-daemon/src/modules.rs`, `api.rs` | MIT OR Apache-2.0 |
| Setup UI | `apps/desktop/src/features/setup/` | MIT OR Apache-2.0 |
| Razer module (OpenRGB port) | `modules/razer-chroma` | **GPL-2.0-or-later** |

## Decisions

**Separate driver processes with a brokered transport.** Each configured
device runs one module process. The module does not open the device: the
daemon opens the HID interface, keeps the handle for the device's lifetime and
executes the module's `write`/`read`/`send_feature`/`get_feature` requests on
it. Reasons, from the actual project:

* The daemon's process-wide FIFO HID gate (DECISIONS.md #11) exists because
  concurrent HID traffic hung the K70 CORE. A module in its own process with its
  own handle would bypass it; the existing `--plugin` system therefore has to
  disable all built-in adapters. Brokered reports pass the same gate, so modules
  and built-in adapters run side by side.
* Handle ownership is unambiguous (daemon), so no two drivers can open one
  device; the module needs no device permissions and never sees paths or
  serial numbers.
* A crash or hang only faults that one device. No Rust ABI is shared.
* The GPL module is a separate program talking to the MIT/Apache service only
  through this generic, documented protocol.

The older stdio *plugin* protocol (`--plugin`, PLUGIN_SYSTEM.md) stays as is;
modules are a new, separate protocol because they need transport calls.

**Package format.** A flat zip with `module.json` (id, version, family,
protocol versions, target triple, executable, license, device matches and the
SHA-256 of every other file), the executable, the license text and
`PROVENANCE.md`. Flat and small enough to validate completely before anything
is written.

**First controller family: Razer Chroma (OpenRGB `RazerController`).** A Razer
Cobra (1532:00A3) is attached to the development PC and not yet supported by
freeRGB; the family is plain USB HID feature reports (no SMBus/driver), one
protocol covers 125 products, and the K70/LINK/ASUS work is unaffected.

## Module protocol, version 1

UTF-8 JSON lines on stdin/stdout, one message per line, at most 256 KiB.

| Host request (`{"v":1,"id":N,"method":…,"params":…}`) | Answer `result` |
|---|---|
| `hello` | `{id, version, family, api}` – must be `api: 1` |
| `identify {interface}` | `{supported, model, kind, reason}` – table lookup, no I/O |
| `initialize {interface}` | capabilities `{name, kind, layout, firmware, restore, zones}` |
| `frame {rgb}` | `true`; `rgb` = base64 of `r g b` bytes for all LEDs in layout order (one message per frame, never per LED) |
| `status` | `{ok, detail}` |
| `shutdown {final_color}` | `true` after the device is released |

Errors: `{"v":1,"id":N,"error":{"code":…,"message":…}}`, codes `unsupported`,
`busy`, `permission`, `timeout`, `disconnected`, `protocol`, `internal`.

While a request is open, the module may send transport calls
`{"v":1,"call":M,"op":"send_feature","data":"<base64>"}` (ops `write`, `read`,
`send_feature`, `get_feature`, ≤4096 bytes, ≤512 per request); the host answers
`{"v":1,"call":M,"data":…,"len":…}` or `{…,"error":…}`. The interface is
described by VID, PID, interface number, usage page, usage and product string.

Host limits: 2 s per request including all transport calls (10 s for
initialize), bounded read/write queues, a child that stops reading is a
timeout; children are killed after a fault and restarted by the engine's
exponential back-off (2–60 s); after 5 consecutive failures the device stays
faulted until the user retries. EOF on stdin means "release the device".
Frames use the engine's existing latest-frame slot: a slow device skips stale
frames instead of queueing them. The module starts with an empty environment
(only `SystemRoot`/`windir`), no console window, its install directory as the
working directory.

Writing a module: depend on `freergb-module-api`, implement
`sdk::Module` (`info`, `identify`, `initialize`, `frame`, `shutdown`) and call
`sdk::run`. Add `--list-devices` printing the catalog device matches. See
`modules/razer-chroma` and the fixture in
`crates/freergb-modules/src/bin/freergb-module-fixture.rs`.

## Catalog and detection

`catalog.json` (schema 1) lists modules with id, version, family, protocol
versions, license, origin (project, URL, commit), device matches (VID, PID and
optional interface/usage page/usage — every given value must match), one
package per target triple (asset name or https URL, size, SHA-256), the
corresponding source archive and dependencies (`requires`). A `sequence`
number must never decrease (rollback protection); `relevant_vendors` decides
which unmatched devices are shown as "not yet supported".

Matching runs locally. The service downloads the whole catalog file once per
start or on request; no device list is ever sent. Only HID enumeration is
used — no bus scans. States are kept apart: *candidate* (relevant vendor, not
matched → "Not yet supported", one card per model), *supported* (catalog or
installed module match), *ready* (module initialized the device). Interfaces
a built-in adapter drives are never claimed by a module. Installed modules
carry their device matches in `module.json`, so matching and startup work
without a catalog.

## Trust chain and installation

1. Ed25519 public key(s) compiled into the daemon
   (`crates/freergb-modules/keys/release-keys.txt`).
2. Detached signature `catalog.json.sig` over
   `"freeRGB module catalog v1\0" || catalog.json`.
3. The signed catalog fixes size and SHA-256 of every package.
4. `module.json` inside the package must equal the catalog entry (id, version,
   family, target, protocol, devices, license) and lists the SHA-256 of every
   file; no other files may exist.
5. The hash of the verified `module.json` is stored in `state.json`; before
   each module start all files are hashed again.

A hash obtained from the download location alone is never trusted. A build
without a key can only run already installed modules (`--module-dev-key HEX`
adds a development key and is logged).

Installation: download to `.downloads/<sha>.part` with timeout, size limit,
three attempts for network errors (not for bad data), cancel removes the
partial file. Verify, extract into `.staging-*` next to the target, check every
file, rename into `<store>/<id>/<version>/`, then replace `state.json`
atomically. Leftover staging/partial files are removed on start. The previous
and the last version that initialized a device are kept;
`modules.rollback` returns to them. Nothing updates on its own: a newer
catalog version is only installed inside a setup the user starts.

Store: `%LOCALAPPDATA%\freeRGB\modules` (Linux `$XDG_DATA_HOME/freergb/modules`);
setup state and catalog cache next to `state.json` in the config directory.

## Service options

| Option | Default |
|---|---|
| `--module-source` / `FREERGB_MODULE_SOURCE` | `github:leotrax3d/FreeRGB@modules` (release tag `modules`; private repo needs `FREERGB_GITHUB_TOKEN`). Also `https://…` base URL or a local directory. |
| `--module-dev-key HEX` | none |
| `--no-modules` | modules enabled |

## Setup flow

Phases per card: Detected → Waiting → Downloading (measured bytes) → Verifying →
Setting up → Connecting → Ready, plus Failed (cause + own retry), Not connected,
Not yet supported (with "Copy device info": IDs, interface, usage, product
string, freeRGB/catalog version — no serial, no path). Cards of one shared
module show the same numbers; the overall bar counts each package once.
Unmeasurable phases show an activity indicator, never a percentage.
Downloading does not mean ready: a device is ready only when the engine reports
it connected (30 s limit, then "did not respond in time").

* First start: full-window wizard while `first_run` is true. Skip, cancel and
  "continue with working devices" are always possible.
* Afterwards: a panel in Devices for new devices (same flow, "Not now"
  dismisses), failed and not-connected ones; unsupported devices and installed
  modules (with "Use previous version") below the device grid.
* Re-plugging a configured device just reconnects it.
* Closing the GUI does not stop jobs; on reopening the phases are read back.
* Light test: explicit button, chosen ready devices, one soft white pulse of
  2.4 s (no flashing), then the running effect continues — the previous
  lighting is always the service's own state, so it is restored exactly.

## Build, sign and release

One-time (project owner):

```bash
cargo run -p freergb-module-tool -- keygen --out module-signing.key   # prints the public key
```

Put the printed public key into `crates/freergb-modules/keys/release-keys.txt`
and commit it; store the content of `module-signing.key` as the repository
secret `FREERGB_MODULE_SIGNING_KEY` and keep the file offline.

Release: the **Hardware modules** workflow (`.github/workflows/modules.yml`)
builds Windows and Linux x86_64 packages, the source archive (and checks that
it builds standalone and that the OpenRGB table regenerates identically), adds
the module to the existing catalog, signs and verifies it. Tags
`module-<id>-v<version>` only produce artifacts. A manual run with
`publish: true` uploads to the `modules` release, created as a **draft**; it
becomes visible to freeRGB only after a person publishes it.

Local equivalent:

```bash
cargo build --release -p freergb-module-razer-chroma -p freergb-module-tool
T=target/release/freergb-module-tool
target/release/freergb-module-razer-chroma --list-devices > devices.json
$T pack --id razer-chroma --version 0.1.0 --family razer-chroma --license GPL-2.0-or-later \
  --target x86_64-pc-windows-msvc --executable target/release/freergb-module-razer-chroma.exe \
  --devices devices.json --file modules/razer-chroma/COPYING --file modules/razer-chroma/PROVENANCE.md \
  --out out/freergb-module-razer-chroma-0.1.0-x86_64-pc-windows-msvc.zip
$T source --root . --manifest modules/razer-chroma/source-workspace.toml --path Cargo.lock \
  --path LICENSE-MIT --path LICENSE-APACHE --path crates/freergb-core --path crates/freergb-module-api \
  --path modules/razer-chroma --path scripts/openrgb --out out/freergb-module-razer-chroma-0.1.0-source.zip
$T add --catalog out/catalog.json --name "Razer Chroma (OpenRGB port)" --package out/…windows-msvc.zip \
  --source-package out/…source.zip --origin-project OpenRGB \
  --origin-url https://gitlab.com/CalcProgrammer1/OpenRGB --origin-commit bbc77470… --vendor 1532=Razer
$T sign --key module-signing.key --catalog out/catalog.json
$T verify --pubkey <hex> --dir out
freergbd --module-source out --module-dev-key <hex>     # test against the local directory
```

### GPL obligations for modules

Every binary package is listed together with its source archive in the signed
catalog and uploaded next to it; the archive contains the module, the shared
crates it builds against, a standalone workspace manifest, `Cargo.lock`, the
generator script and the license files. Each package contains `COPYING`
(GPL text) and `PROVENANCE.md`. Binaries are not bit-for-bit reproducible
across build directories (embedded paths); the archive is the complete
corresponding source either way.

## Adding a module

1. New directory `modules/<id>` (own crate, license of the ported code).
2. Port the controller family, keep the upstream behaviour, record source
   files, commit, changes and test status in `PROVENANCE.md`; generate data
   tables with a script where possible.
3. Report everything not ported as unsupported with a reason.
4. Simulated tests for every report builder and one full sequence.
5. `source-workspace.toml`, `module-info.json`, add the id to the workflow.
6. Hardware validation with the user, logged below.

## Verification

| Area | Evidence |
|---|---|
| Report bytes, CRC, Cobra init/frame/shutdown sequence, all layouts | `cargo test -p freergb-module-razer-chroma` (simulated) |
| Signatures (tampered, wrong key, missing key, missing domain prefix), matching, unsafe/tampered zips, atomic install, failed install keeps old version, modification detected before start, rollback, incompatible protocol | `cargo test -p freergb-modules` |
| Shared module downloaded and counted once, reuse of installed modules, reconnect without new setup, not-connected, offline (cached catalog, installed module), catalog rollback refused, bad package cause | same |
| Real child processes: frame bundling, frame size, crash, hang deadline, restart limit, empty layout, unplug | `cargo test -p freergb-modules --test host` |
| Signed catalog → download → install → verify → installed razer-chroma process → simulated Cobra | `cargo test -p freergb-modules --test e2e -- --ignored` (after building the module) |
| Source archive builds standalone; table regenerates from the pinned commit | Hardware modules workflow (also run locally) |
| GUI selection/size/progress/error logic | `npm test` in `apps/desktop` |

### Hardware validation log

*Not performed yet.* The Razer Cobra port is **ported and simulation-tested,
not confirmed on hardware**. Open steps (each with the user watching, other
Razer/RGB software closed): detect the Cobra card; set up from a locally
signed catalog; confirm initialization, a red/green/blue frame, the light test
and unplug/replug; check that Windows allows feature reports on the mouse
collection (interface 0, usage 1/2) through hidapi from the daemon.

## Known limitations

* The release signing key exists (public key in `keys/release-keys.txt`); until
  the `modules` release is published, the default source reports "catalog unavailable".
* Only Windows and Linux x86_64 packages; Linux needs udev access to the
  hidraw node (not yet in `packaging/linux` rules for Razer).
* Razer: no keyboard-layout devices, no custom/ARGB matrix devices, no
  receiver-paired devices, no hardware effect modes; Blade 14 keep-alive not
  ported.
* No sandbox: a module runs with the user's privileges (but never needs device
  permissions). Authenticity rests on the signing key.
