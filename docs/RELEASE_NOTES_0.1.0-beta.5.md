# freeRGB 0.1.0-beta.5

First public beta candidate, on top of [0.1.0-beta.4](RELEASE_NOTES_0.1.0-beta.4.md).
This is a beta: a large part of the hardware support has **not** been tested on real
devices. Read "Untested hardware" before connecting anything you care about.

## Highlights since beta.4

- Signed in-app updates: the updater now pins the freeRGB release public key and
  verifies a detached Ed25519 signature for every installer
  ([UPDATES.md](UPDATES.md)). Releases without a matching `.sig` asset are refused.
- Tauri desktop GUI with cooling profile export/import, forget action for setup
  devices, empty/loading states and shared Headers setup with motherboard vendor
  conflict handling.
- Classic GUI: shared-locale i18n layer and RPC error-code map.
- Native API 2 SMBus backends (opt-in) and the module policy broker.
- Core Gigabyte Fusion 2 USB variants; many OpenRGB families ported as downloadable
  modules; official effect catalog.
- Fan-control safety: cooling drains on safety revocation, hwmon resume is gated;
  udev rule restricted to `pwmN`/`pwmN_enable`.
- Legal/privacy: Beta labelling in app, installer and docs, safety and liability
  texts, community privacy switches, notices and source offer, path redaction.

## Untested hardware

Only the owner's machine has been used for hardware checks. Everything else is
implemented from protocol references and fixtures only.

- **Fan control** (see `promts/fancontrol/07-hardware-acceptance.md`,
  [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md), [FAN_COVERAGE.md](FAN_COVERAGE.md)):
  the full acceptance session (baseline, write, stop-all, pump protection, GPU via
  NVML, AMD GPU and storage) is not complete. Only channels recorded as
  `hardware-verified` in FAN_COVERAGE.md are verified; all other boards, chips,
  Corsair iCUE LINK variants and GPUs are untested. Fan control keeps its consent and
  channel-identification gates; use them.
- **Motherboard header RGB** ([MAINBOARD_RGB.md](MAINBOARD_RGB.md),
  [MAINBOARD_RGB_DEVICES.md](MAINBOARD_RGB_DEVICES.md)): ASUS Aura is the only board
  family exercised (PRIME X870-P WIFI). MSI, Gigabyte, ASRock and other detectors,
  SMBus/DRAM paths and the Native API 2 backends have no real-hardware results.
- **USB and peripheral modules** ([DEVICES.md](DEVICES.md)): keyboards, mice, mouse
  mats, headsets, coolers, AIOs, lightbars, monitors, Corsair, Razer, Cooler Master and
  most other families are protocol-ported with wire fixtures but not tested on the
  devices. Pipelined Bragi streaming is experimental.
- **Cobra / other pending hardware tests** listed in HANDOFF.md remain open.

## Known gaps

- The installer and executables are **not Authenticode-signed**. Windows SmartScreen
  will warn. The Ed25519 update signature is independent of Authenticode and only
  protects the in-app updater.
- Windows installer lifecycle (UAC, upgrade from beta.4, rollback) was built but not
  executed on a clean VM for this candidate.
- UI translations beyond English and German (S18) are machine translations and
  unreviewed.
- Linux packages (`.deb` etc.) are built by CI only and are not part of this
  candidate; Linux support is not validated on real hardware.
- No live version-to-version update test through the updater has been run.
- Hardware modules and plugins are trusted native code, not sandboxed.
