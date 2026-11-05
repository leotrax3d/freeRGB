# Prompt 09 — AMD GPU fan control on Windows and safe storage temperatures

Read `00-common-rules.md` first. Branch: `cooling/amd-gpu-storage` from `main`
(after prompt 08's PR is merged, merge `main` again before the PR).

## Why
The owner's PC reports an AMD GPU (`/gpu-amd/5`, fan + control via LHM); only
NVIDIA (NVML) can be controlled today, so GPU acceptance (prompt 07 block C) is
impossible on the owner machine. Storage temperatures were removed from the
protected reader because LHM storage enumeration blocked fan telemetry
(`docs/COOLING_ACCEPTANCE.md`, 2026-11-05).

## Tasks
1. **AMD on Windows via ADLX** (driver-provided, runtime-loaded like NVML; FFI in
   its own crate `crates/freergb-gpu-adlx`): detect fans, read speed/RPM, take
   manual control, set duty, release to driver auto. Same safety as NVML:
   journal before first write, release on stop/pause/drop/crash/failed write,
   floor `max(40 %, driver minimum)`, stale/failsafe input -> 100 %.
   Register it as a backend in the existing GPU controller; no second controller.
2. **Decide LHM vs ADLX**: LHM already exposes `/gpu-amd/*/control`. Document
   which path is used for control (prefer the vendor API; LHM GPU controls stay
   read-only) and why.
3. **Storage temperatures without blocking fans**: read NVMe/SATA temperatures
   outside the protected fan reader (separate unprivileged path, e.g. Windows
   storage query IOCTL / WMI `MSFT_PhysicalDisk` / LHM in its own short-lived
   process with a hard timeout). A hang there must never delay motherboard fan
   snapshots. Re-add them to the sensor catalog.
4. Simulated backends and tests for take/set/release/crash and for a storage
   reader that hangs; update `docs/COOLING_SOURCES_GPU.md`, `docs/INSTALL.md`.
5. Add owner test steps (AMD GPU, storage temps) to `07-hardware-acceptance.md`
   block C.

## Done when
Simulated tests pass, CI green, PR open; nothing claims hardware acceptance.
