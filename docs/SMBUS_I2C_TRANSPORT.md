# SMBus and I2C transport proposal

Status: **design only; awaiting owner approval** (2026-10-24). No transport,
manifest schema, driver, discovery probe or hardware access is enabled by this
document. Existing module protocol v1 remains unchanged. Repository source and
documentation language is English.

## Scope and recommendation

Add a daemon-owned, capability-limited transport, first against a simulated bus.
Keep general motherboard SMBus disabled. Prefer the installed vendor GPU driver
for GPU I2C where its documented APIs support the required board bus. Offer
PawnIO only as a separately approved optional motherboard backend. There is no
universal driver-free Windows user-mode SMBus interface: reusing an installed
GPU driver avoids an additional driver, not the need for a driver altogether.
Do not infer RGB reachability from GPU model or API availability.

Reference OpenRGB revision: `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
as recorded in `OPENRGB_PORTING.md`. Exact non-HID detector lists, transaction
recipes and maximum SDK protocol version still require inspection at that pin.
They are not established by the current HID-only inventory.

## Backend evaluation and distribution

| Backend | Reach and constraints | License / distribution assessment | Proposed disposition |
|---|---|---|---|
| PawnIO | Optional signed kernel driver plus controller-specific Pawn modules; existing LHM integration does not establish SMBus support or authorize new bus access. Verify exact controller module and arbitration first. | Driver source is GPL-2.0-or-later with an interface exception; PawnIOLib and official modules have separate LGPL terms. Official binary/installer terms differ from source terms. Audit the pinned artifacts, replacement/source obligations and exception before integration. Existing project policy downloads the official installer after explicit opt-in; do not bundle it. | Optional later; licensing and backend selection require approval. |
| WinRing0 | Broad privileged hardware access through a known vulnerable driver. | Common upstream source distributions use a modified BSD license; exact artifact notices would still need checking. Permissive licensing does not address the vulnerability. | Reject; no installation, fallback or security exclusions. |
| NVIDIA NvAPI I2C | Uses installed NVIDIA driver; public I2C read/write APIs have port, addressing and transfer constraints. Display DDC access does not prove access to an internal RGB controller. Undocumented extended APIs require a separate review. | Public NVIDIA NVAPI repository is MIT; preserve notices for the selected files/libraries. Installed proprietary GPU driver is a separate component; do not redistribute it. | First GPU candidate, public API only, subject to board capability evidence. |
| AMD ADL I2C | Uses installed AMD driver; `ADL_Display_WriteAndReadI2C` supports specific lines and bounded transactions. Verify capability per board and driver. | ADL SDK has its own EULA, not an assumed MIT grant. Published Schedule A restricts redistribution of headers/examples to object code and forbids redistribution of some documentation. Resolve FFI/header and packaging terms before shipping. Do not bundle GPU drivers. | Candidate blocked on exact SDK license review and approval. |
| Linux i2c-dev | Existing kernel adapter exposed through `/dev/i2c-*`; needs permissions and adapter capability checks. Kernel drivers or firmware may already own an address. Never force ownership or unbind a kernel driver automatically. | Kernel implementation is GPL-2.0; Linux UAPI syscall-note headers permit normal user-space use. An independent MIT Rust ioctl client need not incorporate GPL implementation code. Avoid copying GPL convenience-library code; audit any selected dependency separately. | Later optional backend; no automatic bus scan. |

Sources checked 2026-10-24:

* [PawnIO license and interface exception](https://github.com/namazso/PawnIO/blob/master/README.md),
  [PawnIOLib](https://github.com/namazso/PawnIOLib),
  [official module integration and licensing](https://github.com/namazso/PawnIO.Modules/wiki/Using-PawnIO-Modules).
* [Microsoft: WinRing0 vulnerability](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/microsoft-defender-antivirus-alert-vulnerabledriver-winnt-winring0).
* [NVAPI SDK](https://github.com/NVIDIA/nvapi),
  [library license](https://github.com/NVIDIA/nvapi/blob/main/License.txt),
  [public I2C API](https://docs.nvidia.com/nvapi/group__i2capi.html).
* [ADL SDK](https://github.com/GPUOpen-LibrariesAndSDKs/display-library),
  [ADL EULA](https://gpuopen-librariesandsdks.github.io/adl/ADL%20SDK%20EULA.pdf).
* [Linux i2c-dev interface](https://docs.kernel.org/i2c/dev-interface.html),
  [Linux licensing rules](https://docs.kernel.org/process/license-rules.html).

These are backend assessments, not clearance for unresolved redistribution.
The MIT/Apache daemon must not incorporate GPL OpenRGB implementation code.
Derived controller implementations remain separate GPL module programs with
source archives, provenance and notices. IPC separation alone is not a universal
license determination; resolve uncertain dependencies before implementation.

## Broker and admission

Only the daemon (or its narrowly privileged backend helper) opens an adapter.
Modules receive opaque device capabilities, never OS paths, serial numbers,
raw handles, arbitrary PCI configuration access or a bus enumeration primitive.
Adapter identity is resolved internally; public PCI matching uses only
`ven/dev/subven/subdev`. No serial-number query is needed.

All built-in and module accesses to one physical bus enter a process-wide FIFO
admission service, following DECISIONS #11. Aliases from different backends must
resolve to the same bus key; where this cannot be proven, refuse concurrent
ownership. Hold admission across an entire approved multi-operation transaction,
including bank selection and restoration. Bound operations, bytes, elapsed time
and queued work; cancellation must release admission after safe completion or
fault the bus. Different buses may proceed independently. This cannot serialize
firmware or other applications: require conflicting RGB software to be stopped
and use available OS/vendor arbitration. Never claim exclusive hardware safety
from a daemon mutex alone. Coordinate any shared PawnIO/LHM backend as well.

## Authorization policy

The verified package's signed `module.json` declares an explicit capability
allowlist: backend class, exact PCI matches, 7-bit addresses, operations,
register widths/ranges, length limits and permitted values/masks for sensitive
registers. Missing rules deny access. Check the signature and canonical policy
before starting the module, then validate every request in the daemon. A module
cannot widen its capabilities or nominate an arbitrary adapter.

Hard daemon policy overrides signed module declarations:

* Deny every write to SPD addresses `0x50..0x57` and page selectors `0x36/0x37`.
  Apply this to byte, word, block, raw I2C, combined transfers, restore paths and
  backend-specific calls. Normalize addresses once; reject shifted, reserved,
  overflowing and ten-bit forms rather than accepting aliases.
* Raw transfer payloads can encode writes even when described as reads. Permit
  only audited typed register-read recipes; deny page-select commands and raw
  write messages to the forbidden addresses. A register-pointer phase of a read
  needs explicit review, never an unrestricted raw-transfer exception.
* Discovery uses only reviewed read sequences matching pinned OpenRGB behavior.
  No blind address sweep, SMBus quick command or speculative identification
  writes. If OpenRGB detection needs writes, mark that path blocked for review.
* Register checks include every byte of a block, overflow, auto-increment,
  bank/page state and implicit backend side effects. Reject before dispatch.
  Allowlisting a numeric register alone is insufficient for banked controllers.
* Preserve built-in ownership: K70 CORE `1b1c:1bfd/1bff`, iCUE LINK
  `1b1c:0c3f/0c4e`, ASUS Aura USB, Gigabyte Fusion 2 USB and ASRock Polychrome
  USB must not be claimed by these modules; reconcile with `registry.rs`.

Native executable modules remain trusted programs, not a sandbox. Broker policy
protects this transport boundary; it cannot stop an elevated malicious module
from opening hardware independently. Do not launch controller modules elevated.

## Proposed additive protocol (not approved)

Negotiate a new module API version and explicit transport capabilities at hello;
retain v1 HID behavior. Old hosts reject unsupported API requirements before
initialization. Do not silently add accepted fields or calls to v1. Sign the new
manifest fields with the package and validate them before matching a device.

Proposed methods: `smbus_read_byte_data`, `smbus_write_byte_data`,
`smbus_read_block_data`, `smbus_write_block_data`, and typed `i2c_read_register` /
`i2c_write_register`. Word operations, repeated-start combinations and bounded
transaction recipes must be separately advertised and allowlisted. Distinguish
SMBus count-prefixed blocks from I2C fixed-length blocks; specify byte order,
PEC support, STOP/repeated-start semantics and backend limits in the eventual
wire specification. No generic arbitrary-message passthrough in the first version.

Requests contain a device capability, operation, address, register, bounded data
or read length and correlation ID. The host derives bus and policy from the
capability. Responses contain bounded bytes/counts or typed errors: denied,
unsupported, busy, NACK, timeout, disconnected, cancelled, faulted. No implicit
retry of a possibly completed write. A timeout does not prove no bytes were sent.

Manifest PCI entries must explicitly declare all four numeric identifiers or
explicit wildcard semantics. Prefer exact subsystem matches; broad vendor-only
claims cannot authorize writes. Define matching ambiguity and built-in precedence
before implementation. The eventual JSON schema and wire examples require owner
approval; this document deliberately does not change either.

## Consent, failure and recovery

Default is off, including reads and discovery. Present candidate metadata from
non-invasive OS enumeration, then obtain per-device GUI consent before any bus
probe. Warn about hangs, interference and possible persistent hardware damage;
record consent scope and policy version. Installation consent is separate from
device access consent. Reconnect or changed identity must not inherit a different
device's permission. First physical reads/writes and hardware tests additionally
require explicit owner authorization.

Before mutation, capture only approved readable volatile state. Use a bounded,
controller-specific restore recipe under the same policy and bus admission.
Do not restore flash, SPD or unknown state. No generic reset, bus recovery pulse
or write retry. On NACK, disconnect, timeout or child failure: stop frames, revoke
the capability, cancel queued work and attempt restoration only when safe and
explicitly specified. If an in-flight call cannot be interrupted, isolate it in a
helper and quarantine the bus; killing a helper does not undo hardware writes.
Unconfirmed restoration remains a visible fault requiring manual intervention.

Log consent, module/version, policy digest, sanitized PCI IDs, operation class,
address/register, byte count, duration, outcome and restore outcome. Do not log
paths, serials, SPD dumps or unbounded payloads. Rate-limit repetitive failures.

## Implementation and evidence after approval

1. Implement pure policy and a simulated register bus in the MIT/Apache daemon,
   with no native backend enabled. Model bank state, byte order, block counts,
   repeated starts, NACK, short replies, partial writes, hangs and disconnection.
2. Test signed-policy bounds and deny-before-dispatch for every operation,
   particularly SPD/page-select alias and raw-transfer bypass attempts. Test FIFO,
   alias serialization, atomic recipes, cancellation, restore failures and old-host
   version rejection. Mock traces are ordered byte-exact transactions.
3. Inspect the pinned upstream non-HID registrations; generate large GPU tables
   reproducibly, resolve constants and conditional registrations, and preserve
   source evidence. ASUS Aura SMBus/ENE, ASRock SMBus, Gigabyte RGB Fusion SMBus
   and MSI GPU are candidates, not claims of support. Port reviewed families into
   independent GPL modules, each with golden transaction traces against the mock.
4. Leave RAM deferred. `DEVICES.md` records Corsair Vengeance RGB DDR5 at
   `0x18/0x1A`; these RGB addresses do not relax SPD protection. NVIDIA RTX 5060 Ti
   (Gigabyte) and AMD RX 9060 XT are available test candidates, not verified devices.
5. Select a native backend only after separate licensing/driver approval. Hardware
   tests require explicit consent and a dated device/procedure/result log.

For code changes run `cargo fmt --all -- --check`, workspace clippy with
`--all-targets -- -D warnings`, workspace tests and independent module archive
builds. A documentation-only proposal does not establish those results.

## Compatibility and follow-on work

The requested `scripts/openrgb/compat-matrix.mjs`, `OPENRGB_COMPATIBILITY.md`
and `openrgb-compatibility.json` do not exist at this baseline. The existing
`inventory.mjs` scans only `REGISTER_HID_DETECTOR*`; its 121 families / 1179
registrations / 1156 VID/PID entries are inventory counts, not verified support.
Do not extrapolate non-HID device counts from them.

The future generator must inventory I2C bus, PCI, serial and generic detector
registrations at the pin. Each device row needs upstream name, family, transport,
IDs, implementation, functions and evidence. States: `not-implemented`,
`blocked:<reason>`, `implemented` (code and simulated tests), `hardware-verified`
(dated user test in `DEVICES.md`). Missing evidence stays open. No status changes
are made by this proposal; per-transport before/after counts are not yet available.

Upstream drift should compare the explicit pin with a resolved master commit,
record both SHAs and report added/changed/removed registrations plus controller
file changes. Preserve conditional macros and unresolved expressions in the
report. Never automatically port or raise compatibility status.

Further authorized work remains:

| Transport / family | Proposed route | Current evidence / state |
|---|---|---|
| OpenRGB SDK server and optional client | Clean-room MIT Rust from a written protocol specification; loopback `127.0.0.1:6742`, bounded framing, no external bind without explicit setting/approval. Validate `ORGB`, little-endian fields, device index, packet ID and payload length; pin protocol versions before coding. Remote sources need identity namespaces and loop prevention. | `not-implemented` in this work; exact pinned protocol coverage not yet inspected. |
| USB serial: BlinkyTape, HYTE CNVS | Broker serial ports in daemon; GPL modules for derived controllers. Independent MIT implementation only with documented independent protocol evidence. | `blocked:serial-transport`; exact per-device inventory pending. |
| Philips Hue, Nanoleaf, Yeelight, LIFX, Govee | Prefer independent MIT adapters where an official public protocol permits it; otherwise separate GPL port with provenance. Credential storage, discovery and rate limits need per-family design. | `not-implemented` in this work; per-family licensing not established. |
| E1.31, WLED/DDP | Candidate independent MIT implementations from published specifications; confirm document terms and exact protocol variants first. | `not-implemented` in this work; per-family inventory pending. |
| libusb-only | Daemon-owned endpoint broker; installation/OS driver implications reviewed separately. GPL controller modules for derived code. | `blocked:usb-transport`; inventory pending. |
| Linux LED class | MIT sysfs backend using documented kernel ABI, explicit device consent and kernel ownership. | `not-implemented` in this work; inventory pending. |
| Windows device-driver families | Review each driver's API, privileges, license and distribution; no generic raw IOCTL forwarding. | `blocked:driver-review`; inventory pending. |

This table is a planning queue, not the requested complete generated device
matrix or a completed clean-room protocol specification.

## Owner approval requested

Approve the simulation-first broker/policy design and a separately negotiated
module API extension before implementing it. Recommended sequence: mock and
policy tests; reviewed GPU GPL modules; public NvAPI backend after capability
validation; optional PawnIO motherboard support only after artifact/license
review. Keep RAM deferred, WinRing0 excluded and SDK binding loopback-only.
ADL license uncertainty requires a separate decision. No driver installation,
real bus access, hardware tests or external SDK listener is authorized here.
