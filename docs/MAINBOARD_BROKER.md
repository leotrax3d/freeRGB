# Protected read-only mainboard broker

Status: read-only service with explicit Setup opt-in. Protected installation,
standard-user readings, updates, failed-start rollback and removal passed isolated
SCM acceptance with compile-time synthetic sensors (run 39469765979); run 39470499850
also passes unauthorized-account denial and isolated child-crash recovery. This does not
release motherboard fan control or validate real LHM/PawnIO hardware. No API method, GUI setting or API version
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
`READ_DATA | WRITE_DATA | READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE` (`0x00120083`). In particular,
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

Before sending a request the client verifies SYSTEM pipe ownership, binds its PID
via `GetNamedPipeServerProcessId` to the running service reported by SCM, and checks
SCM's exact quoted image path, own-process type and LocalSystem account. It also
verifies that image and its parent installation are protected. This works when a
standard user cannot query the SYSTEM process image directly. The extra pipe
READ_CONTROL right is only for checking ownership, not modifying its ACL. An identically
named pipe from another executable is not trusted. No service means the old reader
prerequisite behavior; access denial, a busy/spoofed server or transport faults do
not silently bypass the broker. There is no automatic elevation.

Cached upstream `age_ms` is increased using a monotonic clock from the original
reader request start. Relaying never refreshes measurement age. Sensor values
older than 5 seconds become null; an expired whole snapshot becomes faulted with
empty hardware. The daemon's existing stricter custom-sensor freshness checks and
full identity/value validation remain in effect.

## Verification and remaining activation work

`BrokerDeployment.cs` adds explicit-consent elevated maintenance, with the fixed
service image quoted in SCM, LocalSystem/automatic start, a protected installing
account SID, an administrator-owned exclusive maintenance lock and rejection of
foreign service registrations/owners. There are no runtime-selected service names
or target directories. The bootstrap must capture the initiating user's SID before
UAC (including a different administrator's credential prompt).

The service build embeds the reviewed package manifest and the exact helper
SHA-256/size. Maintenance validates a pre-elevation hash of its own image, verifies
the helper against the embedded identity and verifies LHM/notices against the
embedded manifest. Each source is held read-only without write/delete sharing
through copying or extraction. It never elevates user-installed LHM DLLs. Runtime
DLLs are freshly extracted from the pinned official archive into a protected stage.
Only admins/SYSTEM can modify deployment directories; users can read/execute. New
files/directories are explicitly owned by Administrators, independent of token
default-owner policy. Signing of our initial distribution is still outstanding;
this binding does not turn an unsigned initial download into a trusted publisher.

An existing owned service is stopped and its process is awaited before switching
directories. The old tree is retained until the new service starts. A failed swap,
registration or start restores the previous files and prior running state. A failed
rollback preserves the protected backup and reports administrator repair. Owned
removal stops/waits/deletes only our service/tree; shared PawnIO/.NET are untouched.
Maintenance must run from the per-user installer copy, outside the protected tree,
so it does not lock the files being updated/removed. Recovery after power loss
between directory renames remains a separate acceptance case; no claim of an
atomic filesystem/SCM transaction is made.

Protected `installation.json` is version 1: `owner_sid`, `protocol_version:1`,
`helper_sha256`. It lives under Program Files beside the service, not in the GUI
configuration directory. It must agree with protected registry owner state. This
file is never a permission to execute a user-supplied path.

`scripts/test-mainboard-deployment.ps1 [-VerifyRuntime]` exercises disk-backed
rollback for running/stopped/new installations, stop/swap/start failures, verified
file immutability, ownership/consent gates and optional real pinned runtime/license
extraction without SCM/hardware access. `scripts/test-mainboard-service.ps1`
requires a disposable elevated GitHub Windows runner, refuses an existing broker,
creates a temporary standard user and tests SCM installation, normal-user readings,
failed-start rollback, version change, tamper/foreign-owner rejection and removal.
Its helper is a **compile-time** synthetic worker, not a production runtime switch;
no LHM hardware code or driver is invoked. Production packaging uses Production
build mode. That acceptance does not validate real sensor labels or fan controls.

Parent `mainboard-maintenance-result.json` version 1 records `action`, `status`,
`message`, `updated_at_utc`, protected against user writes and readable for UAC
failure diagnosis. It and the serialized maintenance lock remain after owned
service removal. Setup's app-directory installation marker is written only after
successful maintenance; reboot/cancellation/failure is explicit. Maintenance is
re-run only with new mainboard opt-in. Interactive uninstall removes the owned
service first and aborts on UAC cancellation/failure; silent uninstall uses an
already elevated owner token or refuses to prompt and preserves a retryable
application installation. Default installer acceptance also exercises this real
uninstall hook with a synthetic protected service and an already elevated token.

`scripts/test-mainboard-broker.ps1` compiles both binaries and a no-hardware fixture
with the .NET Framework compiler. It exercises real Windows pipes: minimal DACL,
exclusive ownership, fake server identity rejection, fixed commands, strict reply
bounds/UTF-8, request/reply timeouts, persistent-handle reconnect after a timed-out
client, acknowledgment, cache aging and unprotected worker/payload rejection. No
service is registered, no driver is loaded and no elevation is requested. CI and
Package run this fixture; the Rust helper transport regression remains separate.

The installer now includes the maintenance executable and invokes it only through
explicit mainboard opt-in/UAC. Remaining acceptance:

* Real wizard/UAC cancellation, manufacturer driver/.NET dialogs and reboot paths.
* Native reader crash/hang/exit cleanup, concurrent clients and power-loss repair
  across directory switches. Fixture reads/SCM success do not establish native
  hardware lifecycle safety. Never load service code from the writable app/cache.
* Real LHM/PawnIO enumeration across motherboard families and confirmed physical
  channel/source labels. The selected ASUS/NCT6701D channel has limited hardware acceptance; see COOLING_ACCEPTANCE.md.

Broad fan/header pairing, PWM/DC mode and verified minimums remain gated.
CHA_FAN2 curve/calibration and same-version maintenance results are recorded in COOLING_ACCEPTANCE.md. Motherboard RGB/ARGB uses a
different backend; see `MAINBOARD_RGB.md`.

Current integration and hardware/release evidence: [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md).
