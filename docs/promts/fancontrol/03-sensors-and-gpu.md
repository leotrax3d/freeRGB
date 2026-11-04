# Prompt 03 — Sensor providers, text-file sensors and GPU fans

Read `00-common-rules.md` first. Branch: `cooling/sensors-gpu`.
Independent of prompts 01/02 (touches providers and sensor sources, not the
controller).

## Goal
Every temperature source FanControl offers can drive a curve, and GPU fans can
be controlled and released safely.

## Tasks
1. **Sensor providers:** CPU (package/CCD/core), GPU (core/hotspot/memory),
   motherboard, storage (NVMe/SATA), liquid temperature from existing Corsair
   LINK telemetry — via the optional LHM provider on Windows and hwmon/drm/nvme
   on Linux. Each source has identity, unit, freshness and a stable ID across
   restarts.
2. **Text-file sensors:** user-configured file path, parse number, bounds,
   freshness timeout; stale/invalid -> treated as missing (curves go 100 %).
   Bounded read size, no shell, no network paths by default.
3. **GPU fan control:** NVIDIA (NVAPI/NVML) and AMD (ADLX on Windows, amdgpu
   hwmon on Linux): capability detection, take control, set duty, guaranteed
   release to driver auto on stop/crash/fault (journal like motherboard
   channels). Intel Arc read-only unless an API supports control.
   Separate protected process if a vendor SDK must be loaded.
4. Sensor graphs (combinations, correction, rolling average) accept all new
   sources; existing daemon-owned custom sensors are extended, not replaced.
5. Simulated providers for CI; document required optional components in
   `docs/INSTALL.md` (nothing mandatory).

## Done when
All sources selectable in curves via API and Tauri GUI; GPU control has
simulated tests for take/set/release/crash; owner test steps (owner GPU) listed
for prompt 07.
