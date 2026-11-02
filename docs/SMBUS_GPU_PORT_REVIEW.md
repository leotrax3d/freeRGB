# GPU controller source review before porting

Inspected local OpenRGB checkout at verified HEAD
`bbc77470a7cba56af60989e54ba910d5ac66a3ac` on 2026-10-24. No native backend or
hardware was opened. Findings below are source facts, not device support claims.
All referenced controller sources declare GPL-2.0-or-later; any derived port
belongs in an independently built GPL module, not the MIT/Apache daemon.

| Family / source | Finding | Port prerequisite |
|---|---|---|
| `MSIGPUController/MSIGPUController` | Detector file contains 83 PCI registrations. It restricts NVIDIA to port 1 and AMD to an AMD GPU I2C bus. Register access uses SMBus byte-data reads/writes. | Preserve backend/port restrictions before enabling discovery; current module identity contains PCI IDs but does not prove a port. Treat this count as registrations, not unique devices. |
| `GigabyteRGBFusion2BlackwellGPUController` | Detector file contains 32 PCI registrations. Identification sends a 64-byte raw I2C query beginning `10 01`, then reads four bytes. Color/mode commands also use 64-byte packets. | Existing typed register transport has no raw messages and a 32-byte limit. The discovery write violates the current read-only detection policy. Keep discovery disabled pending explicit review; do not split packets or silently widen the protocol. |
| `GigabyteRGBFusionGPUController` | Color and mode commands consist of sequences of SMBus write-byte operations without register operands; address `0x62` adds four padding bytes. | Requires an audited command-stream recipe. A register-write operation does not preserve these wire semantics. |

MSI v1 implementation details needing byte-exact tests: current mode, speed,
brightness and colors are read from registers; frames are limited to roughly
30 Hz, write register `0x26` first, and then update `0x30..0x32` for the primary
color or `0x27..0x2c` for fade-in endpoints. Mode/brightness/speed changes are
separate from frame updates. Register `0x3f` is a persistent save operation and
must not enter an ordinary frame or restore allowlist. Register `0x36` on the MSI
RGB controller is distinct from forbidden *bus address* `0x36`; the policy must
continue to distinguish register numbers from addresses.

These findings identify prerequisites, not a finished detector inventory for
every GPU family. No family is ported by this review. The compatibility matrix
remains unchanged; in particular none of the 903 GPU entries is promoted.
Before the first GPL port, create a reproducible PCI-table generator preserving
port restrictions, exact subsystem matching and source provenance, then test
the full lifecycle against the daemon model through the real process host.

Explicit approval is still required for native backend selection, hardware
tests and first real GPU writes. A raw-I2C extension or write-based detection
would be a new security/protocol decision and requires a concrete proposal.
