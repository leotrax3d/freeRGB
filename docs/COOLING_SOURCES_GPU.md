# Temperature sources, text-file sensors and GPU fans

FanControl-parity work (prompt 03). Everything here is simulated/unit-tested
unless a hardware record in `COOLING_ACCEPTANCE.md` says otherwise.

## Source catalog

Every Celsius reading the daemon caches becomes a curve/custom-sensor input in
`cooling.sensors.sources` (unchanged shape) and is described in
`cooling.sensors.catalog`:

```json
[{"id":"[\"mainboard.lhm:/amdcpu/0\",\"/amdcpu/0/temperature/2\"]","label":"AMD Ryzen 7 / Core (Tctl/Tdie)",
  "provider":"mainboard.lhm","unit":"°C","category":"cpu","value":54.8,"age_ms":120,"error":null}]
```

| Category | Windows (optional LHM provider) | Linux (hwmon) |
|---|---|---|
| CPU package/CCD/core | `IsCpuEnabled` (`/amdcpu`, `/intelcpu`) | `k10temp`, `coretemp`, `zenpower` |
| GPU core/hotspot/memory | `IsGpuEnabled` (`/gpu-nvidia`, `/gpu-amd`, `/gpu-intel`) | `amdgpu`, `nouveau`, `i915`/`xe` |
| Motherboard | `IsMotherboardEnabled` (`/lpc/<chip>`) | SuperIO/EC drivers |
| Storage NVMe/SATA | Not via LHM (blocked fan telemetry, 2026-11-05): own unprivileged reader `storage_temps.rs` / `crates/freergb-storage-temp` (storage-query IOCTL, provider `storage`, ids `/storage/<N>/temperature/0`) | `nvme`, `drivetemp` |
| Liquid | Corsair iCUE LINK telemetry (device sensors) | same |
| File | text-file sensors below | same |

IDs are stable across restarts: device IDs come from LHM identifiers, hwmon
device paths (not `hwmonN`) or the native device id; file sensors are
`file:<name>`. The category is a grouping aid derived from those identifiers; it
never decides pump/fan roles. Linux hwmon temperatures previously reached only
`monitoring.sensors.list`; they now also feed curves and custom sensors.
Freshness: provider readings older than their stale limit are `null`; the graph
treats inputs older than 3 s as missing (see `CUSTOM_SENSORS.md`).

## Text-file sensors

Stored under `file_sensors` in `control.json`. `cooling.file_sensors.get`
returns `{sensors:{name:{...}}}`; `cooling.file_sensors.save {set}` validates and
saves atomically (`-32602` on error, saved state unchanged).

```json
{"sensors":{"ambient":{"path":"C:\\temps\\ambient.txt","scale":1,"offset":0,
  "min_c":-20,"max_c":150,"timeout_ms":10000,"allow_network":false}}}
```

* At most 32 sensors; names `[A-Za-z0-9_-]{1,64}`; absolute paths only.
* UNC/`//` network paths are refused unless `allow_network:true`.
* Each second the daemon opens the file directly (no shell), reads at most
  4097 bytes, and parses the first number (split on whitespace, `;`, `,`).
* Missing file, not a regular file, > 4096 bytes, non-UTF-8, no number,
  non-finite, outside `min_c..max_c` after `value*scale+offset`, or a
  modification time older than `timeout_ms` (500..3600000) → `value:null`
  with an `error` in the catalog. Curves using it go to 100 %.

## GPU fans

`cooling.gpu.list` → `{started,paused,backends,fans:[{fan,floor_percent,assignment,applied_percent,fault}]}`.
`cooling.gpu.set` takes `{fan_id,mode:"manual",percent}` or
`{fan_id,mode:"composite",output}` (a `cooling.curve_set` node) and requires the
accepted safety notice (`-32006` otherwise). `cooling.gpu.release {fan_id?}`
returns one or all fans to driver auto. `cooling.fans.pause` also pauses GPU fans.

Backends:

| Vendor | Backend | State |
|---|---|---|
| NVIDIA (Windows/Linux) | NVML, runtime-loaded from `%SystemRoot%\System32\nvml.dll` / `libnvidia-ml.so.1` (driver-provided, nothing bundled); FFI isolated in `crates/freergb-gpu-nvml` | Implemented, **not hardware-tested**; setting speed needs admin/root |
| AMD Linux | `amdgpu` hwmon `pwm1`/`pwm1_enable` through the existing hwmon provider | Covered by `cooling.hwmon.*`; not hardware-tested |
| AMD Windows | ADLX manual fan tuning, runtime-loaded from `%SystemRoot%\System32\amdadlx64.dll` (driver-provided, nothing bundled); FFI isolated in `crates/freergb-gpu-adlx` | Implemented; read-only discovery checked on the owner PC (RX 9060 XT: controllable, minimum 15 %); **control not hardware-tested** |
| Intel Arc | none | Read-only temperatures |

Safety (same pattern as the motherboard providers):

* fan id journaled to `gpu-control-journal.json` *before* the first write; on
  the next start every journaled fan is released to driver auto (crash path);
* release, pause, daemon shutdown (`Drop`) and every failed write release to
  driver auto; a failed write also records a fault and drops the assignment;
* duty never below `max(40 %, driver minimum)`; manual values below are refused;
* composite output missing, older than 3 s or fail-safe → 100 %;
* at most 16 assignments; unknown or non-controllable fans are refused.

### AMD on Windows: ADLX, not LHM

LHM also lists `/gpu-amd/*/control`, but those controls stay read-only in
freeRGB: the vendor API is the supported path, reports the driver's own
limits (speed range, zero-RPM) and lets the curve be restored exactly.
ADLX tunes a fan *curve*, not a raw duty: a duty is written as a flat curve
(every state at the same speed, temperatures unchanged), clamped to the
driver's speed range and the 40 % floor by the controller. The ADLX boundary
also rejects non-finite/out-of-range duties and invalid driver ranges, requires
`take_control` before a duty write, and rounds upward to the driver's speed
step without crossing its maximum. Incomplete/unreadable curves are errors;
they are never saved as a partial restoration capture. `take_control` captures the current
curve and zero-RPM state and disables zero RPM while held; `release` restores
them. After a crash (journal, no capture in the new process) `release` applies
the driver's default curve (`IADLXManualFanTuning1`) and only without that
interface `ResetToFactory` for that GPU's tuning. `cargo run -p
freergb-gpu-adlx --example adlx-discover` lists fans read-only.

### ADLX sensor source (S7, simulated tests only)

`freergb_gpu_adlx::Adlx::sensors()` exposes a current read-only sample per GPU:
`AdlxSensors { gpu_id, temperature_c, hotspot_c, fan_rpm }`. IDs use the same
`adlx:<unique id>` prefix as fan IDs, including GPUs without manual fan support.
The monitoring service is optional; its absence does not disable fan discovery.
This calls the driver's current-metrics API without enabling background tracking
or modifying tuning. Interfaces follow the public
[GPUOpen ADLX monitoring header](https://github.com/GPUOpen-LibrariesAndSDKs/ADLX/blob/master/SDK/Include/IPerformanceMonitoring.h).

Every metric is checked against its support bit and getter result. Unsupported,
failed or implausible readings are `None` (Celsius outside -20..150, non-finite
temperatures, RPM outside 0..100000). Zero RPM is a valid reading. A failed poll
never reuses a previous value. Callers must timestamp each sample and apply the
catalog's freshness policy. A tuning curve's maximum speed is not live duty;
ADLX fan discovery now reports `duty_percent: null` rather than presenting that
curve target as a measurement. Actual RPM is available through `sensors()`.

The new source API is not yet wired into the daemon's sensor catalog: existing
Windows GPU temperature inputs continue to come from LHM. This source increment
does not change the controller, recovery journal, supervisor or consent path.
Fake-vtable tests cover readings, unsupported/failed polls, invalid values,
interface ownership, complete curve capture and duty/range validation. They do
not load the installed ADLX runtime or provide new hardware acceptance evidence.
Owner follow-up: compare core/hotspot/RPM with AMD Software, including zero-RPM
and an unsupported iGPU; then run prompt 07 block C for control/restoration with
the owner present. No hardware commands were run for S7.

### Storage temperatures

Drive temperatures come from `StorageDeviceTemperatureProperty` on
`\\.\PhysicalDriveN` opened with no access rights (works for every user, no
LHM). Each poll (5 s) runs on its own thread with a 3 s timeout; a hung
storage driver only marks the readings stale, never delays a fan snapshot,
and never starts a second query. Readings older than 15 s have no value, so
curves using them go to their fail-safe. `cargo run -p freergb-storage-temp
--example storage-temps` prints them.

Open: NVML is loaded into the daemon (it is part of the driver, not a bundled
SDK). Writes need elevation, so in normal per-user installs NVIDIA fans list
but `set` fails with an explanatory error. ADLX may also need elevation for
tuning writes on some drivers; such failures surface the same way. A protected
elevated GPU worker (like the motherboard broker) remains future work.

Tests: `gpu_cooling::tests` (take/set/release, composite fail-safe, write fault,
pause/drop/crash-journal recovery, refusal), `freergb-gpu-adlx` (range clamp,
flat curve, vtable sizes, missing runtime, fake-interface metrics and curve-read
failures, duty floor/step validation), `storage_temps::tests` (hanging
driver), `freergb-storage-temp` (descriptor parsing), `sensor_sources::tests`,
`control::tests::file_sensors_feed_curves_and_invalid_files_are_missing`,
`apps/desktop/src/features/cooling/sourceModel.test.ts`.
