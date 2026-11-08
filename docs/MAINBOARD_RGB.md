# Motherboard RGB/ARGB (ASUS Aura USB, Gigabyte RGB Fusion 2 USB, ASRock Polychrome USB)

This controls lighting, not fan power, PWM/DC, pump duty or temperatures. It needs
the existing OS HID driver, not PawnIO. Mainboard SuperIO fans use a different
provider.

## Supported controllers

See [MAINBOARD_RGB_DEVICES.md](MAINBOARD_RGB_DEVICES.md) for the list of known USB controllers.


| Family | USB IDs | Interface | State |
| --- | --- | --- | --- |
| ASUS Aura motherboard USB | `0b05:18f3`, `0b05:1939`, `0b05:19af`, `0b05:1aa6`, `0b05:1bed` | `ff72`/`00a1` | Driven (fixed RGB, 12 V headers, up to 4 ARGB headers, Gen1 ARGB selection) |
| Gigabyte RGB Fusion 2 USB (IT5702/IT8297) | `048d:5702`, `048d:8297` | `ff89`/`00cc` | Driven (D_LED1/D_LED2 ARGB headers) |
| Gigabyte Fusion 2 variants | `048d:5711`, `048d:8950`, `0414:a100` | `ff89`/`00cc` | Identified only (more headers/other calibration) |
| ASUS Aura addressable controllers | `0b05:1867`, `0b05:1872`, `0b05:18a3`, `0b05:18a5` | any | Not driven by the core; the GPL module `modules/asus-aura-usb` (installed through device setup) can claim them |
| MSI Mystic Light USB | `1462:7b00`–`1462:7eff`, `1462:1720` | any | Core identifies only; GPL module implements 112/162-byte paths; 185/761-byte paths missing (see audit below) |
| ASRock Polychrome USB | `26ce:01a2`, `26ce:01a6` | any | Driven (per-LED; ARGB1/ARGB2/ARGB3 up to 100 LEDs each, 12 V and onboard zones as fixed LEDs) |
| SMBus/I²C motherboard RGB (older ASUS/MSI/Gigabyte/ASRock boards, ENE/Aura SMBus, Polychrome SMBus) | — | — | **Not supported** (no SMBus access in freeRGB) |

"Identified only" describes the built-in adapter path: it does not write to those controllers. Separately installed GPL modules can implement a subset; see the audit below.

## OpenRGB motherboard coverage audit (S24)

Snapshot: OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, as recorded in
[openrgb-compatibility.json](openrgb-compatibility.json) and
[OPENRGB_COMPATIBILITY.md](OPENRGB_COMPATIBILITY.md). The detector inventory comes
from `scripts/openrgb/compat-matrix.mjs`; `inventory.mjs` is HID-focused and is
not sufficient on its own for SMBus coverage. This audit also checked the pinned
upstream `Controllers/` sources for `DEVICE_TYPE_MOTHERBOARD` and their detectors
(local reference checkout HEAD matched the snapshot).

Scope includes onboard RGB and motherboard RGB/ARGB headers, including OEM
internal USB controllers. GPU, DRAM, laptop keyboards, external hubs and Aura
peripherals are excluded even when they share a controller-family directory.
The DebugController synthetic motherboard is excluded. Framework and legacy
SuperIO RGB are included so generic `REGISTER_DETECTOR` paths are not lost.
ASRock's one SMBus detector dispatches to three protocols, listed explicitly below;
ENE's DRAM/GPU registrations and ASRock's GPU registrations are excluded.

States describe the **direct-colour software baseline**, not complete OpenRGB
feature parity or hardware verification: **done** means an implemented direct
path for the listed controller; **partial** means only some variants/paths work
or identification exists without the full protocol; **missing** means no RGB
implementation. All hardware acceptance remains open. Any future hardware test
must be entered only in
[07-hardware-acceptance.md](promts/fancontrol/07-hardware-acceptance.md);
no hardware or bus access was performed for this audit.

The generated matrix automatically promotes VID/PID matches from module tables
to `implemented`; that is inventory evidence, not proof of a working protocol.
In particular, its Gigabyte module claim is stale: the module refuses initialization.
Its MSI family claim does not distinguish the runtime packet variants. Native
ASUS ownership covers all five motherboard IDs despite the matrix overriding
only `0b05:19af`. Source behavior takes precedence in the audit below.

### Family coverage and missing pieces

The selection contains **16 families and 148 deduplicated detector entries** (not board-model counts).
Family totals: **4 done, 2 partial, 10 missing**.

| OpenRGB family | Entries | Transport | State | Implementation / missing piece |
| --- | ---: | --- | --- | --- |
| `ASRockPolychromeUSBController` | 2 | USB HID | **done** | Native `asrock_polychrome.rs`: direct RGB/ARGB and release restoration. No missing direct-colour piece; vendor effect UI is outside this baseline. |
| `ASRockSMBusController` | 1 | SMBus/I2C | **missing** | SMBus transport, motherboard/vendor checks and firmware dispatch to ASR RGB LED, Polychrome V1 and V2; all three controller protocols/layouts. |
| `AsusAuraUSBController` | 5 | USB HID | **done** | Native `asus_aura.rs`: fixed RGB and up to four configurable ARGB headers. No missing baseline direct-colour piece; previous effect/Gen1 state readback and restoration remain unavailable. |
| `ENESMBusController` | 1 | SMBus/I2C | **missing** | SMBus transport, ASUS motherboard address/signature discovery and ENE register protocol with board zones. |
| `EVGASMBusController` | 1 | SMBus/I2C | **missing** | SMBus transport, EVGA motherboard discovery and ACX30 register protocol/layout. |
| `FrameworkDesktopController` | 1 | other (network/serial/OS) | **missing** | Linux cros_ec device access, Framework DMI gate and EC RGB command implementation. |
| `GigabyteRGBFusion2SMBusController` | 1 | SMBus/I2C | **missing** | SMBus transport, board discovery and RGB Fusion 2 SMBus protocol/layout. |
| `GigabyteRGBFusionController` | 1 | SMBus/I2C | **missing** | SMBus transport, board discovery and legacy RGB Fusion register protocol/layout. |
| `GigabyteRGBFusion2USBController` | 5 | USB HID | **partial** | Native `gigabyte_fusion2.rs` drives 048d:5702/8297 D_LED1/D_LED2. Other variants/header calibration and complete IT8297 configuration/effect state machine remain missing; inventory module rejects initialization. |
| `GigabyteSuperIORGBController` | 1 | other (network/serial/OS) | **missing** | SuperIO RGB discovery, chip gates and register protocol; existing fan-control providers do not implement this RGB controller. |
| `HPOmen30LController` | 1 | USB HID | **done** | `modules/hp-omen-30l`: seven-zone direct-colour reports. No missing baseline direct-colour piece; vendor effects are not exposed. |
| `JGINYUEInternalUSBController` | 1 | USB HID | **missing** | HID detector/claim, V1 initialization, colour packet protocol and zone layout. |
| `JGINYUEInternalUSBV2Controller` | 1 | USB HID | **missing** | HID detector/claim, V2 initialization, colour packet protocol and zone layout (separate from V1). |
| `LenovoMotherboardController` | 1 | USB HID | **done** | `modules/lenovo-motherboard-led`: two LEDs via static-as-direct feature reports. No missing baseline direct-colour piece; vendor effects/DMI naming are omitted. |
| `MSIMotherboardController` | 124 | USB HID | **partial** | `modules/msi-motherboard`: runtime 0x52 probe and 112/162-byte paths. 185/761-byte controller protocols/layouts remain missing; VID/PID alone cannot establish the runtime variant. |
| `MSIRGBController` | 1 | other (network/serial/OS) | **missing** | Legacy SuperIO MSI-RGB discovery, chip gates and RGB register protocol; not the USB Mystic Light controller. |

Evidence: native adapters are in `crates/freergb-protocols/src/` with daemon
selection in `crates/freergb-protocols/src/registry.rs`; daemon settings in
`crates/freergb-daemon/src/mainboard_rgb.rs`. Module evidence is the named module's
`src/lib.rs`, `report.rs` and detector tables. MSI's `initialize_with_zones`
accepts 112/162-byte probes; its older table comment mentioning only 112 is stale.
The pinned upstream `ASRockSMBusControllerDetect.cpp` dispatches to
`ASRockASRRGBSMBusController`, `ASRockPolychromeV1SMBusController` and
`ASRockPolychromeV2SMBusController`: each is **missing**, requiring its own
firmware-specific commands and zone mapping after a transport is available.
The generic matrix transport label `other (network/serial/OS)` means cros_ec for
Framework and SuperIO register I/O for Gigabyte/MSI here, not a network protocol.

### Every selected detector entry

Names, IDs and registration macros below are copied from the pinned matrix.
`dynamic` means no VID/PID is encoded by that registration; it does not mean that
arbitrary buses or boards are supported. HID interface/usage qualifications still
apply (consult the upstream detector and local table); matching a VID/PID is
insufficient. Each row inherits the concrete missing piece in the family table.
For MSI every row stays partial because the protocol variant is discovered at
runtime. The two native Gigabyte IDs have a working limited direct path, but
remain partial against the full upstream controller behavior.

| OpenRGB family | Detector name | ID | Registration | State | Missing-piece reference |
| --- | --- | --- | --- | --- | --- |
| `ASRockPolychromeUSBController` | ASRock Deskmini Addressable LED Strip | `26ce:01a6` | `REGISTER_HID_DETECTOR` | done | ASRockPolychromeUSBController family row above |
| `ASRockPolychromeUSBController` | ASRock Polychrome USB | `26ce:01a2` | `REGISTER_HID_DETECTOR` | done | ASRockPolychromeUSBController family row above |
| `ASRockSMBusController` | ASRock Motherboard SMBus Controllers | `dynamic` | `REGISTER_I2C_DETECTOR` | missing | ASRockSMBusController family row above |
| `AsusAuraUSBController` | ASUS Aura Motherboard | `0b05:18f3` | `REGISTER_HID_DETECTOR` | done | AsusAuraUSBController family row above |
| `AsusAuraUSBController` | ASUS Aura Motherboard | `0b05:1939` | `REGISTER_HID_DETECTOR` | done | AsusAuraUSBController family row above |
| `AsusAuraUSBController` | ASUS Aura Motherboard | `0b05:19af` | `REGISTER_HID_DETECTOR_PU` | done | AsusAuraUSBController family row above |
| `AsusAuraUSBController` | ASUS Aura Motherboard | `0b05:1aa6` | `REGISTER_HID_DETECTOR` | done | AsusAuraUSBController family row above |
| `AsusAuraUSBController` | ASUS Aura Motherboard | `0b05:1bed` | `REGISTER_HID_DETECTOR` | done | AsusAuraUSBController family row above |
| `ENESMBusController` | ASUS Aura SMBus Motherboard | `dynamic` | `REGISTER_I2C_DETECTOR` | missing | ENESMBusController family row above |
| `EVGASMBusController` | EVGA Motherboard SMBus Controllers | `dynamic` | `REGISTER_I2C_DETECTOR` | missing | EVGASMBusController family row above |
| `FrameworkDesktopController` | Framework Desktop | `dynamic` | `REGISTER_DETECTOR` | missing | FrameworkDesktopController family row above |
| `GigabyteRGBFusion2SMBusController` | DETECTOR_NAME | `dynamic` | `REGISTER_I2C_DETECTOR` | missing | GigabyteRGBFusion2SMBusController family row above |
| `GigabyteRGBFusion2USBController` | DETECTOR_NAME | `048d:8297` | `REGISTER_HID_DETECTOR_PU` | partial | GigabyteRGBFusion2USBController family row above |
| `GigabyteRGBFusion2USBController` | DETECTOR_NAME | `048d:8950` | `REGISTER_HID_DETECTOR_PU` | partial | GigabyteRGBFusion2USBController family row above |
| `GigabyteRGBFusion2USBController` | DETECTOR_NAME | `048d:5702` | `REGISTER_HID_DETECTOR_PU` | partial | GigabyteRGBFusion2USBController family row above |
| `GigabyteRGBFusion2USBController` | DETECTOR_NAME | `048d:5711` | `REGISTER_HID_DETECTOR_PU` | partial | GigabyteRGBFusion2USBController family row above |
| `GigabyteRGBFusion2USBController` | DETECTOR_NAME | `0414:a100` | `REGISTER_HID_DETECTOR_PU` | partial | GigabyteRGBFusion2USBController family row above |
| `GigabyteRGBFusionController` | Gigabyte RGB Fusion | `dynamic` | `REGISTER_I2C_DETECTOR` | missing | GigabyteRGBFusionController family row above |
| `GigabyteSuperIORGBController` | Gigabyte RGB | `dynamic` | `REGISTER_DETECTOR` | missing | GigabyteSuperIORGBController family row above |
| `HPOmen30LController` | HP Omen 30L | `103c:84fd` | `REGISTER_HID_DETECTOR` | done | HPOmen30LController family row above |
| `JGINYUEInternalUSBController` | JGINYUE Internal USB Controller | `0416:a125` | `REGISTER_HID_DETECTOR` | missing | JGINYUEInternalUSBController family row above |
| `JGINYUEInternalUSBV2Controller` | JGINYUE Internal USB ControllerV2 | `1a86:e30b` | `REGISTER_HID_DETECTOR` | missing | JGINYUEInternalUSBV2Controller family row above |
| `LenovoMotherboardController` | Lenovo | `17ef:c955` | `REGISTER_HID_DETECTOR_PU` | done | LenovoMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light Common | `0db0:0076` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_1720 | `1462:1720` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_3EA4 | `1462:3ea4` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_4459 | `1462:4459` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B10 | `1462:7b10` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B12 | `1462:7b12` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B16 | `1462:7b16` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B17 | `1462:7b17` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B18 | `1462:7b18` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B50 | `1462:7b50` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B85 | `1462:7b85` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B92 | `1462:7b92` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B93 | `1462:7b93` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B94 | `1462:7b94` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7B96 | `1462:7b96` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C02 | `1462:7c02` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C34 | `1462:7c34` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C35 | `1462:7c35` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C36 | `1462:7c36` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C37 | `1462:7c37` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C42 | `1462:7c42` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C56 | `1462:7c56` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C59 | `1462:7c59` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C60 | `1462:7c60` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C67 | `1462:7c67` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C70 | `1462:7c70` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C71 | `1462:7c71` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C73 | `1462:7c73` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C75 | `1462:7c75` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C76 | `1462:7c76` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C77 | `1462:7c77` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C79 | `1462:7c79` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C80 | `1462:7c80` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C81 | `1462:7c81` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C82 | `1462:7c82` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C83 | `1462:7c83` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C84 | `1462:7c84` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C85 | `1462:7c85` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C86 | `1462:7c86` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C87 | `1462:7c87` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C88 | `1462:7c88` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C89 | `1462:7c89` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C90 | `1462:7c90` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C91 | `1462:7c91` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C92 | `1462:7c92` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C94 | `1462:7c94` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C95 | `1462:7c95` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C96 | `1462:7c96` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C98 | `1462:7c98` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7C99 | `1462:7c99` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D03 | `1462:7d03` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D04 | `1462:7d04` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D06 | `1462:7d06` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D07 | `1462:7d07` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D08 | `1462:7d08` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D09 | `1462:7d09` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D13 | `1462:7d13` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D14 | `1462:7d14` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D15 | `1462:7d15` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D17 | `1462:7d17` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D18 | `1462:7d18` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D19 | `1462:7d19` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D20 | `1462:7d20` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D25 | `1462:7d25` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D27 | `1462:7d27` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D28 | `1462:7d28` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D29 | `1462:7d29` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D30 | `1462:7d30` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D31 | `1462:7d31` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D32 | `1462:7d32` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D33 | `1462:7d33` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D36 | `1462:7d36` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D37 | `1462:7d37` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D38 | `1462:7d38` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D40 | `1462:7d40` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D41 | `1462:7d41` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D42 | `1462:7d42` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D43 | `1462:7d43` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D46 | `1462:7d46` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D50 | `1462:7d50` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D51 | `1462:7d51` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D52 | `1462:7d52` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D53 | `1462:7d53` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D54 | `1462:7d54` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D59 | `1462:7d59` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D67 | `1462:7d67` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D69 | `1462:7d69` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D70 | `1462:7d70` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D73 | `1462:7d73` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D74 | `1462:7d74` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D75 | `1462:7d75` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D76 | `1462:7d76` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D77 | `1462:7d77` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D78 | `1462:7d78` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D86 | `1462:7d86` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D88 | `1462:7d88` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D89 | `1462:7d89` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D90 | `1462:7d90` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D91 | `1462:7d91` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D93 | `1462:7d93` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D96 | `1462:7d96` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D97 | `1462:7d97` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D98 | `1462:7d98` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7D99 | `1462:7d99` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E01 | `1462:7e01` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E03 | `1462:7e03` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E06 | `1462:7e06` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E07 | `1462:7e07` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E09 | `1462:7e09` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E10 | `1462:7e10` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E20 | `1462:7e20` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E30 | `1462:7e30` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E32 | `1462:7e32` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E34 | `1462:7e34` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E59 | `1462:7e59` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E66 | `1462:7e66` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E70 | `1462:7e70` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E76 | `1462:7e76` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E80 | `1462:7e80` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E81 | `1462:7e81` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_7E86 | `1462:7e86` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_905D | `1462:905d` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light MS_B926 | `1462:b926` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIMotherboardController` | MSI Mystic Light X870 | `0db0:0076` | `REGISTER_HID_DETECTOR_PU` | partial | MSIMotherboardController family row above |
| `MSIRGBController` | MSI-RGB | `dynamic` | `REGISTER_DETECTOR` | missing | MSIRGBController family row above |

## Configuration (modern GUI: Devices page; Classic: Monitoring → Motherboard RGB; daemon RPC)

Supported controllers are detected and opened at startup like any other device.
`mainboard-rgb.json` (per-user config directory) holds:

- `enabled` — global "Manage motherboard RGB" switch, default **on**;
- per controller (`vvvv:pppp`) up to four headers with `leds` (0..512) and an
  optional `name` shown on the layout canvas. Presets for 8/12/16/18-LED fans
- ASUS Aura headers also take `order` (`rgb` default, `rbg`, `grb`, `gbr`, `brg`,
  `bgr`) and `fallback` (`{"mode":"spectrum"}` default, `{"mode":"off"}`,
  `{"mode":"static","color":"#rrggbb"}`). `order` is the byte order the strip
  expects: if a red test LED shows green and a green one shows red, choose `grb`.
  `fallback` is what the header shows when freeRGB stops: the firmware spectrum
  effect (the previous effect cannot be read back), the firmware "off" effect, or
  one last direct frame in the given colour. Both are volatile: nothing is
  written to flash. Gigabyte and ASRock headers ignore these two fields.
- `mainboard_rgb.get` also returns `detected` per controller: what the controller
  reports (ASUS: which of the four ARGB headers exist and the fixed-LED count; the
  protocol does not report attached strip lengths, so the LED count stays a user
  setting). The modern GUI can export/import the settings as JSON (validated; the
  per-header test pattern is an owner step, see below).

Owner acceptance (needs the real board, not run by the worker): set a header's
LED count, apply a red/green/blue frame, pick the `order` that shows the right
colours, then close freeRGB and check the chosen `fallback`.
  (times chained fans) are offered by the GUI.

RPC: `mainboard_rgb.get`, `mainboard_rgb.save {settings}`; saving releases the
controllers so the next scan reopens them with the new header table. A header
with 0 LEDs is left out of frames and keeps its firmware effect.

If ASUS Armoury Crate / Aura Lighting Service, MSI Center / Mystic Light,
Gigabyte RGB Fusion / Control Center or ASRock Polychrome RGB is running, freeRGB
does not open motherboard controllers and the Devices page shows a warning.
Other devices keep working.

Test override (unchanged flags, now optional):

```powershell
freergb-launcher --enable-asus-aura --asus-argb-leds 24,24,0,0
```

`--enable-asus-aura` opens ASUS Aura even when management is off or vendor
software runs, with the given LED counts instead of the saved settings.

## Gigabyte RGB Fusion 2 USB wire facts

64-byte feature reports, report ID `0xcc`. `cc 60` then a feature read returns the
controller information (product string at 12..40, strip-length classes at 8..10,
colour order words for D_LED1/D_LED2 at 44 and 48: bytes 2/1/0 are the index of
R/G/B within a triple; anything that is not a permutation is rejected before any
mode write). `cc 34 a b c` sets strip-length classes (32/64/256/512/1024 LEDs, two
nibbles per byte) and is only sent when D_LED1/D_LED2 change. `cc 32 mask` disables
the built-in effect of the selected headers (bit 0 D_LED1, bit 1 D_LED2); `cc 32 00`
re-enables the stored firmware effect on release/fault (volatile). Direct colour:
`cc 58|59 offLo offHi count` + up to 19 RGB triples, byte offset little endian.
Never sent: flash save, effect programming, fan or power commands.

## ASRock Polychrome USB wire facts

65-byte output reports (report ID 0), each answered by a 64-byte input report.
`14` header reads: config `02` = LED count per zone (RGB1, RGB2, ARGB1, ARGB2,
PCH, IO, PCB, ARGB3/audio; `0x1e` = absent), `03` = RGB-swap bitmask, `01` =
availability bitmask. `11 zone` reads a zone's effect (mode at 4, colour 5..7,
speed 8). Anything implausible stops before the first write.

Taking control: the LED-count table is written (`15 02 …`) only if the user's ARGB
counts differ (30 is rejected because it equals the absent marker), hardware
RGB swap is switched off (`15 03 00`, it flickers in per-LED mode) and colour
bytes are ordered per zone instead (GRB, or RGB where the original swap bit was
set). Frames: `10 _ ff e3` with LEDs from byte 9 (18 per report), then
`10 _ ff e4` from byte 5 (19 per report), byte 0x40 = `0x65`; zones packed in
order, padded to the sum of zone maxima + 8 LEDs. Release writes back the
original LED-count table, swap byte and each zone's effect (`10 zone mode r g b
speed ff`; a per-LED mode falls back to spectrum cycle). The commit/save command
`0x12` is never sent.

## GPL hardware modules

`modules/integration` (merged here) adds GPL modules, among them
`modules/asus-aura-usb`. The core adapters above take precedence: the hardware
scan never lets a module claim an interface a built-in adapter drives. Modules
are installed per device through device setup (signed catalog), not by
default. The GPL Gigabyte module branch (`module/gigabyte-rgbfusion2`) was not
taken: it is a stub (no controller query, no effect switch, wrong offset field,
no fallback); the core `gigabyte_fusion2.rs` is used instead.

## OpenRGB reference

Protocol facts were checked against OpenRGB commit
`c43424b4ba94748b913a6551c274251a5211510c` (GPL-2.0) as a reference only; no code
was copied. Files: `Controllers/AsusAuraUSBController/AsusAuraUSBControllerDetect.cpp`,
`Controllers/GigabyteRGBFusion2USBController/GigabyteRGBFusion2USBController{.cpp,.h}`,
`.../GigabyteRGBFusion2USBControllerDetect.cpp`, `.../GigabyteFusion2USB_Devices.h`,
`Controllers/ASRockPolychromeUSBController/ASRockPolychromeUSBController{.cpp,.h}`,
`.../RGBController_ASRockPolychromeUSB.cpp`, `.../ASRockPolychromeUSBControllerDetect.cpp`
(unchanged at commit `7182807a288a9e8107a84fbebc44239677f1ea24`, which lists the same ASUS/Gigabyte IDs),
`Controllers/MSIMotherboardController/MSIMotherboardControllerDetect.cpp`.

## Hardware validation

No motherboard RGB controller has been tested on real hardware by the maintainers
for this release; all of the above is verified against mock transports only
(`asus_aura.rs`, `gigabyte_fusion2.rs`, `asrock_polychrome.rs` tests). Please report results with the
"Hardware report" issue template.

## Earlier notes (ASUS Aura)

The option only affects a newly started daemon. Stop an existing daemon explicitly
before changing its startup options; closing the GUI does not stop it. Demo mode
rejects these options. The daemon also supports the same flags for backend testing;
`--no-hardware` never opens this controller. Header counts require the enable flag,
exactly four comma-separated integers in 0..512, default all zero. Zero excludes the
header from direct frames; it is not evidence that no LEDs are attached. Analog
12 V RGB headers each expose a single color slot, while a 5 V ARGB chain has manual
per-LED length. Check the board/device manuals; the electrical connectors differ.

The firmware config table supplies fixed RGB slots and numbers of RGB/ARGB headers,
not fan/strip models or reliable attached LED counts. We accept at most 16 fixed
slots and four ARGB headers. Unsupported tables, missing headers with nonzero user
counts, malformed replies and unsupported report sizes fail before RGB mode writes.
Configuration replies are 64..65 bytes, report marker `ec`, response `30`, with
60-byte table at offset 4. Table offsets `1b`, `1d`, `02` describe fixed slots,
analog headers and addressable headers. Names reflect firmware slot order; physical
connector mapping still needs confirmation.

The worker queries configuration (`ec b0`), then uses volatile direct mode
(`ec 35`, effect `ff`). Direct RGB payloads (`ec 40`) carry 20 RGB triplets per
65-byte output report; final packet applies the channel. ARGB offsets above 255
use the ninth-offset flag. Mainboard/fixed RGB uses direct channel 4; ARGB uses
channels 0..3. Any selected ARGB zone also selects controller-wide **Gen1** via
`ec 52 53 00 01`. This can affect Gen2 detection on other headers, including ones
excluded from frames; it is not isolated to the selected strip. The option takes
ownership of controller lighting and must not run concurrently with another RGB
controller application.

Disconnect/drop attempts firmware spectrum-cycle effect 4 for acquired effect
channels. The previous effect and previous Gen1/Gen2 state cannot currently be
read/restored. This is a known fallback, not restoration of prior user settings.
Failed fallback retains ownership for retry, preventing reacquisition until it
succeeds. No flash/persistent commit (`3f`), shutdown-effect update, fan or pump
commands are implemented. Gen1 selection lifetime and actual fallback behavior
still need hardware validation; use the vendor software to reconfigure afterward
if necessary. This adapter is not automatically enabled by prerequisite installation.

All effects, brightness and canvas regions use the existing daemon API and device
worker, shared by both GUIs. Existing API types are unchanged. The worker publishes
the discovered named layout before rendering; empty pre-discovery frames are
skipped. No protocol logic or extra dependencies are added to either GUI.

## Evidence and acceptance

Independent Rust implementation from documented wire facts; no upstream C++ code
is incorporated. Source reference pinned to OpenRGB commit
`7182807a288a9e8107a84fbebc44239677f1ea24`:
[base protocol](https://github.com/CalcProgrammer1/OpenRGB/blob/7182807a288a9e8107a84fbebc44239677f1ea24/Controllers/AsusAuraUSBController/AsusAuraUSBController/AsusAuraUSBController.cpp),
[mainboard protocol](https://github.com/CalcProgrammer1/OpenRGB/blob/7182807a288a9e8107a84fbebc44239677f1ea24/Controllers/AsusAuraUSBController/AsusAuraUSBController/AsusAuraMainboardController.cpp),
[detection](https://github.com/CalcProgrammer1/OpenRGB/blob/7182807a288a9e8107a84fbebc44239677f1ea24/Controllers/AsusAuraUSBController/AsusAuraUSBControllerDetect.cpp).

Mock transport tests cover malformed tables/no mode writes, packet chunking and
offset overflow, names/selected headers, frame-size rejection, only allowed RGB
commands, fallback and restoration-failure ownership. These are not hardware
acceptance. Real discovery, physical header correlation, Gen1/Gen2 behavior,
colors/per-LED order, fallback, unplug/replug and long-running tests remain open.
