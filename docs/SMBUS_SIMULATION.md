# Simulated bus protocol and broker

The owner approved this simulation phase on 2026-10-24. Code is independent
MIT/Apache Rust; no OpenRGB implementation was copied. This is transport
infrastructure with synthetic tests, not a ported controller family.

## API boundary

`freergb-module-api::bus` defines API 2 independently of the unchanged HID API 1.
`Hello {v:2, api:[2], transport:"simulation"}` must negotiate successfully before
a daemon `bus_simulation::SimulatedBus` can create a session. This is currently
an in-process simulation interface; no stdio v2 process launcher, module SDK bus
client, GUI device opt-in or production setup routing is enabled.

Example JSON call (the capability is assigned by the host):

```json
{"v":2,"call":7,"capability":3,"transfers":[{"op":"smbus_read_byte_data","address":24,"register":32,"data":[],"read_len":1}]}
```

`Session::call_json` bounds each input to 256 KiB before deserializing and
dispatching. This fixture entry point does not provide a process pipe reader.
The reply uses `{v,call,result}`, where serde encodes `result` as
`{"Ok":[[170]]}` or `{"Err":"denied"}`. Unknown fields and operation names
are rejected when deserializing. The existing host's bounded JSON line framing
is not a v2 process integration. No arbitrary JSON-to-native dispatch exists.

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
constructor. Catalog compatibility and production process startup still require
HID API 1, so schema-2 packages cannot be selected/launched through normal setup.
The package tool's existing HID pack command remains schema 1.

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

Next implementation work is verified-package v2 process integration, bounded
wire framing/SDK support, canonical adapter registry, richer error fixtures,
safe controller recipes and reviewed per-family GPL ports. PawnIO/ADL licensing,
native backend selection, first hardware access/tests and an external SDK
listener retain their separate approval gates. RAM remains deferred.
