# Mainboard broker: read-only boundary candidate

Status: implemented and tested as a **service candidate**, not yet installed or
enabled by Setup. This does not release motherboard fan control or solve the
remaining protected-deployment work. No API method, GUI setting or API version
changes. The existing optional helper can consume the broker when it is available;
with no broker it retains its existing direct-reader prerequisite checks.

## Privilege boundary

`freergb-mainboard-service.exe` is intended to run as `freeRGBMainboard`, LocalSystem,
from the fixed 64-bit Program Files directory `freeRGB/MainboardBroker`. It refuses
startup under another identity or path. Its directory, ancestors up to Program
Files, helper, library and other deployed files must have trusted ownership
(SYSTEM, Administrators or TrustedInstaller), no reparse points and no effective
unprivileged write/delete/ACL/ownership grants. Inherit-only rules are not effective
permissions on the checked object. Administrators remain trusted.

The service reads `AllowedUserSid` from the administrator-protected key
`HKLM/SYSTEM/CurrentControlSet/Services/freeRGBMainboard/Parameters`. This is an
installation setting, not a GUI configuration file or RPC parameter. A missing or
invalid account SID fails startup. There is no user-supplied executable, library
path, download, raw I/O, register operation, script or setter in the protocol.

Only the fixed protected `freergb-mainboard.exe --direct-protected-library` child
loads LHM from the protected `library/` directory. The worker switch checks SYSTEM,
its exact image path and installation protection before opening hardware. The
desktop helper cannot use this switch from its ordinary per-user installation.
Only motherboard sensor groups are enabled. There are no fan-control setters and
stock LHM `Close()` is not called because its SuperIO cleanup can write controls.

The service supervises one isolated reader process, uses a 10-second first-read
deadline / 3-second subsequent deadline, polls every second, kills/reaps failed
children and waits 5 seconds between crash/restart attempts. IPC serves only cached
snapshots, separately from native reads. Service stop closes IPC and kills the
child; GUI close never stops the service or daemon. Full SCM/child restart and
shutdown acceptance still needs a disposable Windows installation.

## Local transport, version 1

The fixed byte-mode endpoint is `\\.\pipe\freergb-mainboard-v1`. The initial handle
uses `FILE_FLAG_FIRST_PIPE_INSTANCE`, rejects remote clients and stays open across
client reconnects. Losing exclusive ownership fails closed. The protected pipe
DACL grants SYSTEM/Administrators full control and the configured user only
`READ_DATA | WRITE_DATA | READ_ATTRIBUTES | SYNCHRONIZE` (`0x00100083`). In particular,
it does **not** grant generic write / create-pipe-instance rights to that user.
Everyone/anonymous are not granted access. See Microsoft's
[pipe access-rights guidance](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)
and [CreateNamedPipe flags](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipea).
Clients also specify identification-only security QoS so a rejected server cannot
obtain an impersonation-capable client token.

A connection allows one fixed `snapshot\n` request, one UTF-8 JSON line in the
existing helper version-1 format, then receipt acknowledgment byte `0x06`. No
command batch is evaluated. The acknowledgment prevents disconnect from discarding
an unread reply. Request, response write, acknowledgment and client reply have
2-second deadlines. Pending named-pipe I/O is cancelled before reusing the same
handle; failed cancellation stops the broker. A stalled reader does not extend the
request deadline by sending one byte at a time. EOF, invalid UTF-8, oversized or
unterminated replies fail closed; replies are bounded to 256 KiB.

Before sending a request the client checks the server PID's actual image path via
`GetNamedPipeServerProcessId` / `QueryFullProcessImageName` with limited query rights,
and verifies that image and its parent installation are protected. An identically
named pipe from another executable is not trusted. No service means the old reader
prerequisite behavior; access denial, a busy/spoofed server or transport faults do
not silently bypass the broker. There is no automatic elevation.

Cached upstream `age_ms` is increased using a monotonic clock from the original
reader request start. Relaying never refreshes measurement age. Sensor values
older than 5 seconds become null; an expired whole snapshot becomes faulted with
empty hardware. The daemon's existing stricter custom-sensor freshness checks and
full identity/value validation remain in effect.

## Verification and remaining activation work

`scripts/test-mainboard-broker.ps1` compiles both binaries and a no-hardware fixture
with the .NET Framework compiler. It exercises real Windows pipes: minimal DACL,
exclusive ownership, fake server identity rejection, fixed commands, strict reply
bounds/UTF-8, request/reply timeouts, persistent-handle reconnect after a timed-out
client, acknowledgment, cache aging and unprotected worker/payload rejection. No
service is registered, no driver is loaded and no elevation is requested. CI and
Package run this fixture; the Rust helper transport regression remains separate.

The build generates the service candidate but **the installer does not yet include
or register it**. Before activation, implement and validate all of the following:

* Explicit Setup opt-in describing our own system service, as well as upstream
  components; a protected elevated deployment using verified upstream bytes.
* Protected owner/SID state, service identity/image checks, transactional updates,
  failure recovery and owned-service removal. Never register an image or load a
  DLL from the user-writable application/cache directories as SYSTEM.
* Disposable-machine acceptance for SCM startup, reader crash/hang, concurrent
  normal-user clients, unauthorized accounts, UAC cancellation, upgrade/rollback
  and uninstall. Real LHM/PawnIO operation remains unverified here.

Fan/header pairing, PWM/DC mode, safe minimums, calibration, actual writes and an
independent failsafe remain separate gated work. Motherboard RGB/ARGB uses a
different backend; see `MAINBOARD_RGB.md`.
