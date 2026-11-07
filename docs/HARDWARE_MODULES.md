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
| Redragon mouse module (OpenRGB port) | `modules/redragon-mouse` | **GPL-2.0-or-later** |

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
  serial numbers except during an explicitly permitted read-only identity probe.
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

API 3 cooling plugins (schema-3 packages with a signed sensor/channel/HID policy) are documented in [COOLING_PLUGINS.md](COOLING_PLUGINS.md).

The separate, simulation-only API 2 bus host/SDK and schema-2 package route are
documented in [SMBUS_SIMULATION.md](SMBUS_SIMULATION.md). They do not change this
HID process protocol or enable native bus modules in production setup.

UTF-8 JSON lines on stdin/stdout, one message per line, at most 256 KiB.

| Host request (`{"v":1,"id":N,"method":…,"params":…}`) | Answer `result` |
|---|---|
| `hello` | `{id, version, family, api}` – must be `api: 1` |
| `identify {interface}` | `{supported, model, kind, reason}` – table lookup, no I/O |
| `initialize {interface, zones?}` | capabilities `{name, kind, layout, firmware, restore, zones}`; optional `zones` is a list of `{name, leds}` supplied by user configuration for families whose LED count is not discoverable (for example Thermaltake Riing) |
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

## Hardware effect modes (HID API 1, optional extension version 1)

Modules may implement `sdk::Module::hardware_modes`, `set_hardware_mode`,
`resume_direct` and `save_hardware_mode`. Defaults preserve existing modules.
After successful initialize, the SDK adds `hardware_modes: {version:1,modes:[...]}`
only for a nonempty catalog. Old hosts ignore this field; new hosts send no new
requests to modules that omit it. The HID envelope and package API stay at 1;
bus API 2 and cooling API 3 are unaffected. Unknown extension versions fail closed.

Each descriptor contains `id` (stable module-local string), `name`, `value`
(upstream mode value), `flags`, optional `speed`/`brightness` ranges `{min,max}`,
and `colors:{min,max}`. At most 128 modes, unique bounded IDs, and at most 4096
colors are accepted. Speed and brightness preserve native integer units and
reversed endpoints (e.g. speed min=5/max=1); both endpoints are inclusive.
These are whole-device operations; there is no per-zone selection in version 1.

The contract maps the pinned OpenRGB `RGBController/RGBControllerInterface.h`
mode semantics without importing controller implementation code:

| OpenRGB meaning | Extension representation |
|---|---|
| speed, brightness | Flag bits 0, 4; corresponding range and required selection value |
| left/right, up/down, horizontal/vertical, diagonal | Flag bits 1, 2, 3, 11; selection `direction` values 0..9 in upstream order |
| per-LED, mode-specific, random colors | Flag bits 5, 6, 7; `color_type` 1, 2, 3 (0 means none) |
| color count | Per-LED requires the layout LED count; mode-specific uses the declared inclusive min/max; random/none require an empty color vector |
| manual save | Flag bit 8; enables the separate confirmed save command |
| automatic save | Flag bit 9 is **rejected**: selection must be volatile |
| requires entire device | Flag bit 10; all extension operations already apply to the entire assigned device |

Selection is `{id,speed?,brightness?,direction?,color_type,colors:[{r,g,b},...]}`.
Unsupported parameters must be absent/null; supported speed, brightness and
direction must be supplied. No defaults or silent clamping are inferred.
Unknown IDs, flags, fields, invalid color types/counts and out-of-range values
are rejected before transport I/O by both host and SDK.

Wire request: `hardware_modes {version:1,command:...}`. Commands are
`{op:"list"}`, `{op:"set",selection:...}`, `{op:"direct"}` and
`{op:"save",confirmed:true}`. Responses contain `{catalog,active}`; `active`
is null in direct mode. Initialize/reconnect starts direct with no remembered
selection. The host serves list from the validated cached catalog without I/O.
Set/direct/save run on the device's owning worker between frames, using the
existing daemon-owned HID handle and broker limits. A one-slot command queue
rejects overload; queued commands expire after one second before execution.
The local API waits at most four seconds, without holding the engine mutex.

After acknowledged set, the host suppresses streamed frames until acknowledged
direct; the SDK independently rejects frames while a mode is selected. The
module's direct hook must restore its volatile streaming state. Runtime selection
is not persisted to host state/profiles, replayed on reconnect, or saved during
shutdown. A failed transition faults the module process through existing recovery.

Local API `modules.modes` accepts `{device:<engine-id>,command:<above>}`.
For example, `freergb ctl modules.modes '{"device":"module:fixture:test","command":{"op":"list"}}'`.
Errors follow the existing local API convention: -32602 malformed parameters,
-32004 missing device, -32005 refused/unavailable/busy/timeout. Mutating commands
are refused while lighting is paused or a software conflict is present.

**Saving requires a separate user action and confirmation.** A UI must explain
that device storage will be written and then send `save` with `confirmed:true`.
Missing/false confirmation, no active mode, or missing manual-save capability
is refused. Set, direct, frame, shutdown and reconnect must never save implicitly.
Families whose firmware always saves on selection cannot advertise that path.
Signed native modules remain trusted programs, not a sandbox: a dishonest module
can mislabel its packets. Catalog validation does not prove physical persistence.

Proof is simulation-only: the original MIT fixture in
`freergb-module-fixture` emits synthetic packets, and host tests assert their
exact bytes including a save packet only after confirmation. Daemon tests cover
local RPC and worker serialization without render ticks. See the fixture
[provenance and owner acceptance](../crates/freergb-modules/tests/fixtures/hardware-modes/PROVENANCE.md).
This removes the API blocker for future Redragon, SteelSeries Rival, ViewSonic
and Holtek hardware effects; no family implementation or compatibility row is
promoted by this change.

## Read-only platform identity (API 1 additive extension)

`identify` and `initialize` accept an optional `params.platform` object:
`{"board_vendor":"Fixture Vendor","board_name":"Fixture Board A (FB-1)","product":"Fixture Product"}`.
Each field is an optional string; absent, denied or invalid reads stay absent.
The host snapshots only these labels once per daemon process: Windows reads
`BaseBoardManufacturer`, `BaseBoardProduct`, `SystemProductName` from
`HKLM\HARDWARE\DESCRIPTION\System\BIOS` using read-only registry access;
Linux reads `board_vendor`, `board_name`, `product_name` from `/sys/class/dmi/id`.
Neither requires elevation, a driver, a subprocess or hardware writes.
Unsupported platforms return an empty identity. Labels are trimmed, limited to
256 UTF-8 bytes and rejected on embedded control characters; names are not
case-folded, shortened or inferred from the USB product string.

SDK `identify_with_platform` and `initialize_with_platform` hooks receive this
snapshot; their defaults delegate to the existing identify/zone initialization
hooks. API stays at version 1: old modules ignore the additive field and old
hosts omit it. A module requiring a board identity must fail closed when the
field is missing or does not exactly match its audited configuration. An
identity label is a detection input, not a grant of device access or proof of
protocol compatibility; transport probing still applies. No serials, UUIDs,
firmware tables, registry/sysfs paths or handles are exposed to modules.

The host's `platform::read_with`, `read_sysfs(root)` and
`ModuleDevice::set_platform_identity` allow deterministic injection. Unit and
real-child-process fixture tests cover absent/denied/malformed values, exact
name gating without writes on mismatch, and initialize/frame/shutdown through
the existing HID broker. No family driver is changed by this extension.

For future family work this removes the DMI-input blocker for MSI 761's
35 exact `board_names` configurations at pinned OpenRGB commit
`bbc77470a7cba56af60989e54ba910d5ac66a3ac`, including MAG X870 TOMAHAWK WIFI
(MS-7E51) and MPG X870E EDGE TI WIFI (MS-7E59). The 761 packet implementation
remains **blocked: licence/provenance review (SignalRGB-derived), owner
decision**. Identity availability does not promote any compatibility row.

## Read-only identity probes (extension version 1)

The HID envelope remains API 1. A schema-1 package may opt into the separately
versioned identity extension with a signed `probe` field in `module.json`:

```json
"probe": {
  "version": 1,
  "serial": true,
  "product": true,
  "features": [{ "report_id": 82, "max_len": 200 }]
}
```

The release tool accepts `pack --probe-policy policy.json` (also in release
plans). Permissions are authenticated through the existing signed package digest
and reverified manifest hash. Unknown extension versions, duplicate report IDs,
zero/over-4096 lengths and use with non-HID packages are rejected. Omitting this
field preserves existing manifests, wire messages, module implementations and
startup behavior. Old hosts reject packages containing the extension field;
publish opt-in packages only for hosts that support it.

After `hello`, before `initialize`, and on each reconnect, the host sends
`probe {version:1, interface}`. The result has the same shape as `identify`.
Implement `sdk::Module::probe` using `probe::Hid`; its default delegates to
legacy table lookup. The dedicated trait exposes only `serial_string()`,
`product_string()` and `feature(report_id, len)`. The original `identify`
remains an I/O-free table lookup.

String calls use ops `serial_string` / `product_string` with empty data and
zero length/timeout. Replies contain optional base64 UTF-8 data and byte length;
missing data means no string, while empty base64 means an empty string. Strings
are limited to 1024 UTF-8 bytes. Feature reads use `get_feature`, exactly one
report-ID byte in data, zero timeout and a nonzero length at most the manifest's
maximum (including report ID). Only returned bytes are forwarded. No arbitrary
prefix bytes are accepted. This supports firmware queries that are pure HID
GET_REPORT operations; request/response protocols requiring writes are excluded.

The host enforces these restrictions independently of the SDK. Output writes,
feature writes and input reads are denied during probes. String reads are denied
outside probes. Existing runtime feature-report operations remain unchanged.
The daemon retains the same opened handle and all native calls use the global
HID gate. A refused or failed probe prevents initialization and therefore any
initialization/frame writes. The existing call-count, message-size and 2-second
process deadline apply; a blocking OS HID call retains the existing backend
limitation and cannot be cancelled by the process deadline.

Identity strings are transient module inputs: do not log, persist, echo them in
probe errors/results, or add them to device diagnostics. The host does not copy
probe model/reason fields into device diagnostics and masks probe failures before
exposing them. Backend string errors are replaced with a generic error. Native
process modules remain trusted executables, not a security sandbox.

This API enables future external family work for MSI common VID 0x0DB0/PID 0x0076
virtual-PID extraction from the serial prefix and the read-only 0x52 variant
probe, and Cooler Master ARGB/SmallARGB firmware gates based on exact serial
strings. It does not implement their lighting protocols or add supported devices.
Logitech 0xC547 `ProbeG915ReceiverName` requires two HID++ output writes
(`10 01 03 0E 00 00 00`, `10 01 03 1E 00 00 00`) upstream and remains
blocked for the A6 HID++ workstream. A USB product string cannot substitute for
the paired keyboard's HID++ name. No family module or coverage ledger is changed.
Upstream evidence: OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/MSIMotherboardController/MSIMotherboard185Controller/MSIMotherboard185Controller.cpp`
(constructor), `Controllers/MSIMotherboardController/MSIMotherboardControllerDetect.cpp`,
`Controllers/CoolerMasterController/{CMARGBController,CMSmallARGBController}/`
(`GetVersion` / `GetSerial`), and
`Controllers/LogitechController/LogitechControllerDetect.cpp`
(`ProbeG915ReceiverName`). These references document constraints; no upstream
implementation is copied into the MIT/Apache API or broker.

Owner acceptance: after external family ports adopt the policy, verify their
read-only identity decisions on supported and unsupported firmware without
lighting writes, confirm no serial appears in exported diagnostics, and record
hardware results in each module's PROVENANCE.md. Current evidence is simulated.
## Optional HID timing extension, version 1

The outer HID protocol remains version 1. Existing modules work unchanged:
capabilities without a `timing` member schedule no ticks. New SDK modules add
`"timing":{"v":1,"tick_ms":500}` (or null to disable) to the initialize
result. The host rejects unknown timing versions and intervals outside
500..=10000 ms, including noninteger values. This is independent of API 2
bus modules and API 3 cooling plugins; their scheduling is unchanged.

Implement `Module::tick_interval_ms` and `Module::tick` to receive
`{"v":1,"id":N,"method":"tick"}` through the normal request/reply channel.
The interval is sampled after initialization, so it may depend on detected
capabilities. Tick I/O is brokered on the same assigned handle as frames.
The device worker checks maintenance while idle (100 ms polling granularity)
and before frames. First tick is due one interval after initialize; later
intervals start when the preceding tick completes. Missed ticks coalesce;
there is no catch-up burst. Frames do not postpone ticks. Tick failures and
hangs fault the module and use the existing reconnect policy. Disconnect or
conflict release drops the schedule; reconnect starts a fresh schedule.
This is best-effort timing, not a real-time guarantee: worker execution and
blocking OS HID calls can postpone a tick.

Within an assigned-device runtime request (excluding read-only identity probes),
`Hid::delay_ms(13)` emits
`{"v":1,"call":M,"op":"delay","data":"","len":0,"timeout_ms":13}`.
The host sleeps before acknowledging that call, so the next packet follows
the delay. It holds no HID gate during the sleep. Delays must be positive,
at most 100 ms each, and at most 1000 ms in total per request. They count
toward the existing 512-call budget and the existing request deadline
(2 s, or 10 s for initialize); none extends the deadline. Invalid shape,
negative/zero/oversized values or an exhausted budget abort the request
before a later packet can execute. Delay calls without an assigned device
are rejected. Stop is observed after an in-flight bounded request completes.

Old hosts ignore the optional capabilities member and cannot execute the new
`delay` operation; timing-dependent modules therefore require a host with
this extension. The backward compatibility guarantee is old modules on new
hosts. The SDK's default tick and delay methods keep existing implementations
source-compatible; mock transports explicitly implement delay support when needed.

Simulated proof: `freergb-module-fixture` exercises idle ticks, ordered writes
separated by a 13 ms delay, interval bounds, cumulative-delay refusal and
failed/hung ticks in `freergb-modules/tests/host.rs`. This API unblocks the
MSI motherboard 185 per-LED path (13 ms packet gaps), Corsair Commander Core
keepalive and Thermaltake Riing Quad keepalive for the external port authors.
No family implementation, device table or port ledger status is changed here;
byte-exact vendor packets and owner hardware acceptance remain their work.

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

### One-command local release

`freergb-module-tool release --plan release-plan.json --out out/modules --key <owner-key-path>`
builds locked release targets, packs binaries and corresponding source, updates
the catalog, signs it, verifies hashes and package contents, and writes
`RELEASE_NOTES.md`. It never uploads or publishes. The owner-provided signing key
must be outside the source root and must never be committed.

Use `--dry-run` without `--key` in CI to parse and validate the plan and print
build commands; this does not build, read a key or write output. The plan is
trusted release-maintainer input. Paths resolve from the invoking working
directory; binary/source inputs resolve within the canonical `root`. Source
archive paths must be relative without parent components or symlinks.

Example plan (replace synthetic names and versions with reviewed release inputs):

```json
{
  "root": ".",
  "builds": [{"package": "fixture-module", "target": "x86_64-pc-windows-msvc"}],
  "notes": "Describe changes and hardware acceptance limits here.",
  "steps": [
    {"command":"pack", "id":"fixture", "version":"0.2.0", "family":"fixture", "license":"MIT", "target":"x86_64-pc-windows-msvc", "executable":"target/x86_64-pc-windows-msvc/release/fixture-module.exe", "devices":"devices.json", "cooling_policy":null, "files":["COPYING","PROVENANCE.md"], "out":"fixture-0.2.0.zip"},
    {"command":"source", "root":".", "paths":["Cargo.lock","crates/fixture-module"], "manifest":"source-workspace.toml", "out":"fixture-source.zip"},
    {"command":"add", "catalog":"catalog.json", "name":"Fixture module", "packages":["fixture-0.2.0.zip"], "source_package":"fixture-source.zip", "origin_project":"Fixture", "origin_url":"https://example.invalid/fixture", "origin_commit":"abcdef0", "vendors":[]}
  ]
}
```

Repeat pack per target and add per module. JSON field names use underscores;
`vendors` contains `[numeric_vid, "Name"]` pairs. All output asset names must be
flat and unique, and every produced asset must be consumed by an add step.
Optionally set `previous` to a local complete release directory and
`previous_pubkey` to its trusted hex public key: the tool verifies and carries
forward its catalog and assets before updating entries. Catalog sequence advances
per add; set `FREERGB_CATALOG_DATE` to the reviewed release date.

The destination must not exist. A sibling `.release-staging` directory is reserved
exclusively, and becomes the final directory only after successful verification.
Failures retain staging for diagnosis; inspect and remove it manually before
retrying. A dry run validates plan structure, not file availability, compilation,
licenses or hardware behavior. Unit tests use synthetic package bytes, a fake
builder and a public deterministic test key, including tamper and build failure.

### Owner acceptance steps for local releases

1. Review source provenance, complete corresponding source and license notices;
   capture device metadata through the existing approved process.
2. Run dry-run, then the local command with an offline owner key outside the tree.
   Build all required target toolchains on appropriate hosts.
3. Independently verify with the trusted public key; review catalog sequence,
   asset hashes and generated release notes. Build the source archive standalone.
4. Perform explicit hardware acceptance separately and record unsupported cases.
5. Upload the verified directory to the `modules` draft through the established
   owner workflow; review before publishing. WS4 performs no publication.

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
