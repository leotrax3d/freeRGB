# Native control write-ahead journal candidate

The private native-session candidate now requires a `JournaledNativeControl`
port. Before native acquisition, it durably writes an intent. It clears that
intent only after the owned native release callback returns and ownership is
relinquished. Live control is still not enabled: no production caller/listener
constructs this chain, and the normal snapshot service is unchanged.

## Location and version 1

Protected metadata lives at
`%ProgramFiles%\freeRGB\mainboard-control-recovery.json`, outside the versioned
`MainboardBroker` directory. It is **not** a shared GUI/config-root file, an
installer activation marker or permission to execute a path. Both GUIs remain
RPC-only; no new public method reads/writes this file.

Empty state:

```json
{"version":1,"reservation":null}
```

Before acquiring a selected control:

```json
{"version":1,"reservation":{"id":"0123456789abcdef0123456789abcdef","backend":"lhm_0_9_6","hardware_id":"/synthetic/board","sensor_id":"/synthetic/control/7","pump":false}}
```

IDs in the example are synthetic. The transaction ID is a random GUID in N
format. Hardware/sensor identifiers come from the selected native Control sensor,
whose type/group/reference must match a SuperIO control. They are not inferred
from header labels or copied from unverified GUI annotations. The pump flag
describes that port's restriction; no previous low duty or curve is stored/replayed.
Required exact object shapes, fixed backend/version, boolean role, nonblank
identifiers <=512 UTF-8 bytes without control characters; maximum file size 16 KiB.
Missing file means no prior intent. Corrupt, unsupported, oversized or non-file
state blocks acquisition. The initial candidate permits only **one pending
channel intent** globally; multiple simultaneous controlled channels need a later
backward-compatible format/ownership extension before release.

The pinned Nct677X native adapter may additionally include `recovery` in a pending
entry: `kind:"lhm_nct677x_registers_v1"`, chip name, selected index0..6,
port0..65535, revision/mode/command bytes, and `service_identity`/`worker_identity`
strings (`PID:process-start-UTC-ticks`). Mode must describe original nonzero
automatic mode. The seven-field baseline form is accepted for old fixtures;
production adds both lifetime identities. Older readers reject extended pending
state and therefore cannot silently erase it during rollback. Empty/older journal
records retain their existing format. This is privileged recovery metadata, not
a shared GUI configuration file, and never stores an old requested low duty.

Recovery runs before production snapshots. It identifies the exact sensor/hardware,
chip/index/port/revision and refuses another still-living original worker (PID reuse
is checked against start time; query denial is not proof of death). In the same
worker it retries the original owned adapter. For a dead worker, an already
returned original mode permits a non-writing clear; otherwise register restoration
requires the same registered SYSTEM service lifetime. Only the original selected
restore cache is seeded in the fresh pinned LHM backend, then original mode is read
back before journal clear. Different service/boot lifetime with differing native
mode, legacy missing provenance, unknown hardware or failed readback retain the
intent. No low-duty replay or blind fresh-context SetDefault is used.

## Ordering, locks and failures

Production construction requires the protected SYSTEM worker. Parent/file owners
and write permissions/reparse points are checked using the existing broker path
policy. A fixed global mutex allows only SYSTEM/administrators and rejects an
untrusted preexisting owner/ACL. It serializes journal operations across processes
with bounded waits; an abandoned mutex does not bypass file validation.

Writes use a same-directory temporary file with WriteThrough and `Flush(true)`,
then File.Move for the first state or File.Replace for later states. A successful
reserve must return before any native acquisition. Duties/heartbeats do not
rewrite the file. A failed write therefore blocks acquisition; failed native
acquisition/restoration leaves the intent; a failed clear preserves the pending
record and can be retried without redoing a successful native release.

An unresolved intent blocks any new channel reservation, including after worker
death/restart. Parsing the journal alone never clears it or reacquires the channel;
only the checked protected recovery operation described above can clear it.
Only the exact transaction ID can be completed. A failed intermediate temporary
write before Reserve returns cannot authorize native I/O. Atomic replacement/file
flushing targets process-crash ordering, **not a proven power-loss/filesystem
durability guarantee**; directory metadata flush and abrupt machine loss remain
unvalidated. Administrative deletion/tampering is outside the unprivileged boundary.

## What remains before activation

The protected pinned Nct677X recovery adapter now checks original native baseline
metadata and exact worker/service lifetimes, and restores only the proven original
mode/command as described above. Worker-crash recovery passed on the actual
NCT6701D test board during a fixed100 diagnostic. This does not establish safe
recovery across all chips, service/OS crashes, power loss or blocked drivers.
A fresh LHM context's SetDefault alone is still not original-state proof.
No old low duty may be replayed.

The productive host still needs exclusive lifetime/admission integration and
independent physical fallback acceptance. Upgrade/uninstall
must coordinate with active sessions before stopping or replacing the worker;
maintenance now refuses pending/invalid journal state and holds the reservation
mutex through replacement/removal (see `COOLING_MAINTENANCE.md`), but does not yet
provide that active-session release protocol. Multiple
channels, driver/native hangs, service/OS crash and power loss remain open. These
requirements keep the candidate inactive despite passing synthetic tests.

## Evidence

`test-mainboard-control.ps1` tests durable-before-I/O ordering, duplicate/orphan
rejection, failed journal writes, partial acquisition, failed native restoration,
failed clear/retry and invalid/oversized files. A real subprocess is killed after
its intent is flushed; a fresh journal instance rejects reacquisition before any
synthetic I/O. A real pinned LHM Control object with synthetic events is exercised
through journal -> session -> adapter -> fallback -> explicit release.

The SCM fixture also writes/reads/clears the production protected journal as the
actual SYSTEM worker, then a separate standard user attempts a write and must be
denied without changing the file. This runs only on disposable GitHub Windows;
local tests use temporary directories and synthetic ports, never install a
service/driver and never open Computer/SuperIO. Factory gates assert test-directory
constructors are absent from production. Check the fresh SCM result separately
from the already passed service-client identity fixture.
