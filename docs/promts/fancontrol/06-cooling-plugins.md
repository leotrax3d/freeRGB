# Prompt 06 — Cooling plugins (sensors and controls)

Read `00-common-rules.md` first. Branch: `cooling/plugins`.
Depends on prompt 03.

## Goal
Like FanControl's plugin ecosystem, third parties can add sensors and fan
controls (e.g. AIOs, fan hubs, laptop ECs) without changing the core.

## Tasks
1. Versioned cooling plugin contract reusing the hardware module system
   (`docs/HARDWARE_MODULES.md`): signed package, brokered process, declared
   capabilities (sensor, control, both), policy-limited I/O.
2. Host enforces the same safety as built-in channels: lease, floor, pump
   rules, freshness, fail-safe release on plugin crash/hang.
3. Reference plugins: a virtual test plugin (CI) and one real one built from an
   existing native device (e.g. iCUE LINK fan speeds) to prove the contract.
4. GUI: install/enable/disable plugins in the module manager; plugin channels
   appear like native ones with a source badge.

## Done when
Contract documented; host process tests cover crash, hang, malformed output and
policy violation; reference plugins pass; owner test step for the LINK plugin
listed for prompt 07.
