# Simulated bus protocol and broker

The owner approved this simulation phase on 2026-10-24. Code is independent
MIT/Apache Rust; no OpenRGB implementation was copied. This is transport
infrastructure with synthetic tests, not a ported controller family.

## API boundary

`freergb-module-api::bus` defines API 2 independently of the unchanged HID API 1.
`Hello {v:2, api:[2], transport:"simulation"}` must negotiate successfully before
a daemon `bus_simulation::SimulatedBus` can create a session. The API now also has
a stdio v2 process host and module-side SDK. `SimulatedBus::start_module` is the
explicit host entry point for a verified installed package, exact PCI identity
and consent. Native transport, GUI device opt-in and automatic setup routing
remain disabled.

Example JSON call (the capability is assigned by the host):

```json
{"v":2,"call":7,"capability":3,"transfers":[{"op":"smbus_read_byte_data","address":24,"register":32,"data":[],"read_len":1}]}
```

`Session::call_json` bounds each input to 256 KiB before deserializing and
dispatching. This fixture entry point does not provide a process pipe reader.
The reply uses `{v,call,result}`, where serde encodes `result` as
`{"Ok":[[170]]}` or `{"Err":"denied"}`. Unknown fields and operation names
are rejected when deserializing. No arbitrary JSON-to-native dispatch exists.

## Process protocol

`freergb-modules::bus_host::BusProcess` reuses bounded child pipes (four queued
messages per direction, 256 KiB per JSON line) and process cleanup from the HID
host. The daemon closes stdin then kills/reaps a stuck child within the existing
500 ms grace period. Requests have a two-second shared deadline, or ten seconds
for initialization; each broker call also honors that same deadline and its own
two-second limit. No retry or automatic relaunch follows a v2 fault.

Process requests use a deliberately nested typed method object:

```json
{"v":2,"id":1,"method":{"method":"hello"}}
{"v":2,"id":2,"method":{"method":"identify","params":{"pci":{"ven":4318,"dev":1,"subven":5208,"subdev":2}}}}
{"v":2,"id":3,"method":{"method":"initialize","params":{"pci":{"ven":4318,"dev":1,"subven":5208,"subdev":2},"capability":9}}}
```

`hello` returns `Info {module:ModuleInfo, hello:Hello}`. The host checks exact
package id, version, family, API and simulation transport. `identify` is pure
PCI lookup; unsupported identification ends startup. Only `initialize`, `frame`
and `shutdown` may contain bus calls. A frame contains base64 RGB bytes and must
match the validated layout. Final responses are `{v:2,id,result:{Ok:value}}` or
`{v:2,id,result:{Err:{code,message}}}`. The SDK rejects out-of-order host request
IDs, invalid lifecycle order, malformed/oversized lines and invalid bus replies.

The earlier plain `Call` example above is the in-memory transaction type. On
process pipes it is wrapped with its parent host request ID:

```json
{"request":3,"transaction":{"v":2,"call":1,"capability":9,"transfers":[{"op":"smbus_read_byte_data","address":24,"register":0,"data":[],"read_len":1}]}}
```

Replies to these transactions retain the plain `Reply` format. The host checks
parent request ID and strictly increasing call IDs, at most 512 calls per host
request. It refuses early, replayed, unknown-version or malformed calls. A
rejected transaction fails the host request even if the child hides the error.
Unclean process termination, missing replies and protocol errors revoke the
session and quarantine its shared bus because earlier writes may have completed.
Dropping a live process is also unclean; successful shutdown only revokes that
session. No restore is inferred from process exit.

Module authors implement `bus::sdk::Module` and call `bus::sdk::run`. They receive
a `dyn Bus` supporting bounded transaction batches, never native handles or
paths. EOF ends the SDK immediately; it cannot issue unsolicited restore writes
after the host has disappeared. Controller-specific restoration must be an
explicit shutdown recipe under the existing policy.

Six typed operations exist: SMBus byte-data read/write, SMBus block-data
read/write, I2C register read/write. Addresses and registers are unsigned 8-bit
values, with addresses restricted further to canonical 7-bit `0x08..0x77`.
There is no address-shift conversion on input. Lengths are 1..32 bytes, byte-data
operations require exactly one byte, and a transaction contains 1..32 operations.
SMBus block traces include a count byte; I2C register block traces do not.
Trace bytes omit START/STOP/ACK timing and PEC; reads model a register-pointer
write followed by a repeated-start read. The flat register fixture returns the
requested count; variable device-returned counts, malformed/short responses,
banking and controller-specific framing need later models before a native backend.

## Policy and packages

`PackageManifest` schema 1 remains HID-only with no bus policy. Schema 2 requires
exactly API `[2]`, an empty HID match list and a `bus` policy. That policy contains
`backend:"simulation"`, `flat_registers:true`, exact four-field numeric PCI
matches (`ven`, `dev`, `subven`, `subdev`) and rules. No wildcard matching exists.
Rules allow one address, operation, register interval and maximum length.
For writes every byte must satisfy `byte & !write_mask == write_value`.
Read rules use the same structure but masks do not authorize writes.

Manifest validation rejects forbidden writes even in a publisher policy. The
broker independently rejects SPD/page-select writes before rule lookup. Every
byte in a transfer must fit one rule; separate rules cannot be combined to widen
a transfer. All operations in a batch are validated before any operation runs.

Production packages are authenticated through the existing trusted catalog
signature -> package SHA-256 -> manifest/file hashes chain; parsing a manifest or
extracting a zip alone does not authenticate it. The in-memory simulation API
accepts a host-supplied policy to facilitate tests; it is not a verified-package
constructor. The process route instead obtains `Store::verified_bus` and derives
the session policy from its opaque `VerifiedBus` handle before launching a child.
The file hashes and recorded manifest digest are rechecked on each start.

Catalog schema 1 remains unchanged for existing publishers/clients. An explicit
schema-2 catalog may describe bus-only API `[2]` entries with no HID device claims;
policy and PCI matches remain inside the authenticated package manifest.
`Store::install_bus` is a separate opt-in installation API; its caller must supply
an entry from a signature-verified schema-2 catalog. Installation grants no bus
permission. Normal HID `compatible`, `install` and `verified` routes reject these
packages, and no auto-discovery path selects them. The existing pack command still
creates schema 1. Do not replace a published v1 catalog with schema 2 without a
versioned distribution channel: old hosts cannot parse schema 2.

## Admission and errors

One `SimulatedBus` is one canonical synthetic bus. All its sessions share a FIFO
queue (maximum 64 waiting calls), and admission remains held across each batch.
The caller supplies PCI identity and explicit consent when creating a session;
the module cannot alter either through a call. Capabilities are unique per
session; revocation rejects subsequent calls and cancels queued work. The
simulation caller must share the same bus object for aliases. Native adapter
identity and process-wide physical-bus alias registration are still unimplemented.

Admission and each operation check a two-second deadline and cancellation flag;
waiters recheck within ten milliseconds. Register-model calls are immediate and
do not model an uninterruptible native call. A failure after admission revokes
the session and quarantines the entire bus. Completed writes remain applied.
No retry, generic reset or speculative restoration is issued. Recovery currently
means replacing the synthetic fixture; controller-specific safe restoration is
required before any real controller port is enabled. Panic also releases admission
and quarantines the bus. Debug events contain correlation ID, operation count
and outcome; memory-only byte traces are fixture evidence, not production logs.

## Evidence and remaining work

Tests cover byte-exact traces for all six operations; SPD/page-select rules;
invalid aliases, lengths, wrapping registers and mixed read/write fields;
whole-batch preflight; masks and ranges; consent, exact subsystem IDs, versions,
capability mismatch and revocation; FIFO batch admission; queued cancellation
and timeout cleanup; NACK, disconnect, timeout, partial completion and panic.
Schema tests cover v1/v2 separation and signed-data mutation detection.

Validation on 2026-10-24: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings` and
`cargo test --workspace` completed successfully. Existing ignored fixture tests
were not enabled. A final targeted run passed all 12 broker tests and the new
manifest test. No new controller module/source archive was produced, and no
independent archive rebuild or physical hardware validation is claimed.

No GPU/RAM/mainboard families were ported. Matrix counts before/after are
unchanged: GPU I2C 0 implemented / 0 verified / 903 open; DRAM 0 / 0 / 7;
SMBus/I2C 0 / 0 / 8 open plus 1 blocked. Other transport rows are unchanged.
The generated matrix must not promote devices because a mock broker exists.

Five additional end-to-end tests run a real synthetic child from a signed catalog
and installed schema-2 package against the daemon register model. They cover
initialize/frame/shutdown traces; consent and policy tampering; hidden denials,
crash, hang, oversized/invalid JSON and mismatched request/version; early bus I/O,
wrong identity/API and unsupported identification; and crash after a completed
write with shared-bus quarantine. SDK tests check reply shape, correlation and
limits. Run them with no hardware:

```text
cargo test -p freergb-daemon --features bus-fixture --test bus_process --locked
```

The fixture binary is gated behind `bus-fixture` and excluded from ordinary
builds. These are simulation tests, not hardware acceptance.

Next implementation work is canonical adapter registry, richer error fixtures,
safe controller recipes and reviewed per-family GPL ports. PawnIO/ADL licensing,
native backend selection, first hardware access/tests and an external SDK
listener retain their separate approval gates. RAM remains deferred.

GPU source assessment: see [SMBUS_GPU_PORT_REVIEW.md](SMBUS_GPU_PORT_REVIEW.md).
