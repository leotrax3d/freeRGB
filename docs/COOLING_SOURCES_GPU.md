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
| Storage NVMe/SATA | not enabled in the protected reader (blocked the first snapshot on the owner board, 2026-11-05) | `nvme`, `drivetemp` |
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
| AMD Windows | ADLX | **Not implemented** (temperatures only, via LHM) |
| Intel Arc | none | Read-only temperatures |

Safety (same pattern as the motherboard providers):

* fan id journaled to `gpu-control-journal.json` *before* the first write; on
  the next start every journaled fan is released to driver auto (crash path);
* release, pause, daemon shutdown (`Drop`) and every failed write release to
  driver auto; a failed write also records a fault and drops the assignment;
* duty never below `max(40 %, driver minimum)`; manual values below are refused;
* composite output missing, older than 3 s or fail-safe → 100 %;
* at most 16 assignments; unknown or non-controllable fans are refused.

Open: NVML is loaded into the daemon (it is part of the driver, not a bundled
SDK). Writes need elevation, so in normal per-user installs NVIDIA fans list
but `set` fails with an explanatory error. A protected elevated GPU worker (like
the motherboard broker) and ADLX remain future work.

Tests: `gpu_cooling::tests` (take/set/release, composite fail-safe, write fault,
pause/drop/crash-journal recovery, refusal), `sensor_sources::tests`,
`control::tests::file_sensors_feed_curves_and_invalid_files_are_missing`,
`apps/desktop/src/features/cooling/sourceModel.test.ts`.
