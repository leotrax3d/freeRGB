# OpenRGB backlog

Generated 2026-11-06 from docs/openrgb-compatibility.json (OpenRGB bbc77470). Tags are a first heuristic by transport/family; workers confirm the real blocker before porting. Largest device counts first.

| Family | Not implemented | Transport | Needs |
|---|---|---|---|
| ENESMBusController | 177 | SMBus/I2C, I2C/PCI (GPU), other (network/serial/OS) | A7 (SMBus/I2C) |
| MSIGPUController | 130 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| GigabyteRGBFusion2GPUController | 103 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| GigabyteRGBFusionGPUController | 86 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| AsusAuraGPUController | 74 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| EVGAAmpereGPUController | 60 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| PalitGPUController | 59 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| ColorfulGPUController | 40 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| SapphireGPUController | 33 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| GigabyteRGBFusion2BlackwellGPUController | 32 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| EVGATuringGPUController | 26 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| ZotacV2GPUController | 17 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| WootingKeyboardController | 13 | USB HID | A5? (check) |
| PNYLovelaceGPUController | 12 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| PowerColorGPUController | 11 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| GalaxGPUController | 10 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| ASRockSMBusController | 8 | SMBus/I2C, I2C/PCI (GPU) | A7 (SMBus/I2C) |
| EVGAGP102GPUController | 8 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| HyperXMicrophoneController | 8 | USB HID | none? (check) |
| PNYARGBEpicXGPUController | 7 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| EVGAPascalGPUController | 5 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| QMKController | 5 | USB HID, other (network/serial/OS) | A7 (serial) |
| ZotacTuringGPUController | 4 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| HyperXMousematController | 3 | USB HID | none? (check) |
| HYTEMousematController | 3 | USB serial, other (network/serial/OS) | A7 (serial) |
| LianLiController | 3 | other (network/serial/OS) | A7 (serial) |
| RealtekBridgeController | 3 | USB HID | none? (check) |
| ViewSonicController | 3 | USB HID | none? (check) |
| ZotacBlackwellGPUController | 3 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| AlienwareController | 2 | USB HID | none? (check) |
| AlienwareKeyboardController | 2 | USB HID | A5? (check) |
| AlienwareMonitorController | 2 | USB HID | none? (check) |
| AOCKeyboardController | 2 | USB HID | A5? (check) |
| ClevoKeyboardController | 2 | USB HID | A5? (check) |
| ColorfulTuringGPUController | 2 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| CougarController | 2 | USB HID | none? (check) |
| DuckyKeyboardController | 2 | USB HID | A5? (check) |
| HoltekController | 2 | USB HID | none? (check) |
| HYTENexusController | 2 | USB serial | A7 (serial) |
| KingstonFuryDRAMController | 2 | SMBus DRAM | A7 (SMBus/I2C) |
| MNTKeyboardController | 2 | USB HID | A5? (check) |
| NZXTKrakenController | 2 | USB HID | none? (check) |
| RedSquareKeyroxController | 2 | USB HID | none? (check) |
| SRGBmodsController | 2 | USB HID | none? (check) |
| TrustController | 2 | USB HID | none? (check) |
| ValkyrieKeyboardController | 2 | USB HID | A5? (check) |
| AMBXController | 1 | other (network/serial/OS) | A7 (serial) |
| ArcticController | 1 | other (network/serial/OS) | A7 (serial) |
| AsusTUFLaptopController | 1 | other (network/serial/OS) | A7 (serial) |
| BlinkyTapeController | 1 | USB serial | A7 (serial) |
| CorsairHydro2Controller | 1 | other (network/serial/OS) | A7 (serial) |
| CorsairHydroController | 1 | other (network/serial/OS) | A7 (serial) |
| CorsairPeripheralV2Controller | 1 | USB HID | none? (check) |
| CorsairVengeanceController | 1 | SMBus DRAM | A7 (SMBus/I2C) |
| CreativeController | 1 | other (network/serial/OS) | A7 (serial) |
| CrucialController | 1 | SMBus/I2C | A7 (SMBus/I2C) |
| DDPController | 1 | other (network/serial/OS) | A7 (serial) |
| DebugController | 1 | other (network/serial/OS) | A7 (serial) |
| DMXController | 1 | other (network/serial/OS) | A7 (serial) |
| DygmaRaiseController | 1 | other (network/serial/OS) | A7 (serial) |
| E131Controller | 1 | other (network/serial/OS) | A7 (serial) |
| EdifierHaloController | 1 | USB HID | none? (check) |
| EKController | 1 | USB HID | none? (check) |
| ElgatoKeyLightController | 1 | other (network/serial/OS) | A7 (serial) |
| ElgatoLightStripController | 1 | other (network/serial/OS) | A7 (serial) |
| EspurnaController | 1 | other (network/serial/OS) | A7 (serial) |
| EVGASMBusController | 1 | SMBus/I2C | A7 (SMBus/I2C) |
| FanBusController | 1 | other (network/serial/OS) | A7 (serial) |
| FaustusController | 1 | other (network/serial/OS) | A7 (serial) |
| FrameworkDesktopController | 1 | other (network/serial/OS) | A7 (serial) |
| GigabyteAorusCPUCoolerController | 1 | USB HID | none? (check) |
| GigabyteAorusPCCaseController | 1 | USB HID | none? (check) |
| GigabyteCastor3Controller | 1 | USB HID | none? (check) |
| GigabyteRGBFusion2DRAMController | 1 | SMBus/I2C | A7 (SMBus/I2C) |
| GigabyteRGBFusion2SMBusController | 1 | SMBus/I2C | A7 (SMBus/I2C) |
| GigabyteRGBFusionController | 1 | SMBus/I2C | A7 (SMBus/I2C) |
| GigabyteSuperIORGBController | 1 | other (network/serial/OS) | A7 (serial) |
| GoveeController | 1 | other (network/serial/OS) | A7 (serial) |
| HIDLampArrayController | 1 | USB HID | none? (check) |
| HPOmenLaptopController | 1 | other (network/serial/OS) | A7 (serial) |
| HyperXDRAMController | 1 | SMBus DRAM | A7 (SMBus/I2C) |
| HYTEKeyboardController | 1 | USB HID | A5? (check) |
| IonicoController | 1 | USB HID | none? (check) |
| JGINYUEInternalUSBController | 1 | USB HID | none? (check) |
| JGINYUEInternalUSBV2Controller | 1 | USB HID | none? (check) |
| KasaSmartController | 1 | other (network/serial/OS) | A7 (serial) |
| KeychronKeyboardController | 1 | USB HID | A5? (check) |
| LaviewTechnologyController | 1 | USB HID | none? (check) |
| LEDStripController | 1 | other (network/serial/OS) | A7 (serial) |
| LGMonitorController | 1 | USB HID | none? (check) |
| LIFXController | 1 | other (network/serial/OS) | A7 (serial) |
| LightSaltController | 1 | USB HID | none? (check) |
| LinuxLEDController | 1 | other (network/serial/OS) | A7 (serial) |
| LogitechController | 1 | USB HID | A6 |
| ManliGPUController | 1 | I2C/PCI (GPU) | A7 (SMBus/I2C) |
| MintakaKeyboardController | 1 | USB HID | A5? (check) |
| MSIClutchController | 1 | USB HID | none? (check) |
| MSIRGBController | 1 | other (network/serial/OS) | A7 (serial) |
| MSIVigorController | 1 | USB HID | none? (check) |
| NanoleafController | 1 | other (network/serial/OS) | A7 (serial) |
| NVIDIAIlluminationController | 1 | other (network/serial/OS) | A7 (serial) |
| NZXTHuePlusController | 1 | USB serial | A7 (serial) |
| OKSController | 1 | USB HID | none? (check) |
| PatriotViperController | 1 | SMBus DRAM | A7 (SMBus/I2C) |
| PatriotViperSteelController | 1 | SMBus DRAM | A7 (SMBus/I2C) |
| PhilipsHueController | 1 | other (network/serial/OS) | A7 (serial) |
| PhilipsWizController | 1 | other (network/serial/OS) | A7 (serial) |
| RealtekARGBController | 1 | USB HID | none? (check) |
| RobobloqLightStripController | 1 | USB HID | none? (check) |
| SeagateController | 1 | other (network/serial/OS) | A7 (serial) |
| SkydimoSerialController | 1 | USB serial | A7 (serial) |
| SkyloongController | 1 | USB HID | none? (check) |
| StreamDeckController | 1 | USB HID | none? (check) |
| TForceXtreemController | 1 | SMBus DRAM | A7 (SMBus/I2C) |
| ThrustmasterSolController | 1 | other (network/serial/OS) | A7 (serial) |
| ValveSteamMachineController | 1 | other (network/serial/OS) | A7 (serial) |
| WinbondGamingKeyboardController | 1 | USB HID | A5? (check) |
| WitmodKeyboardController | 1 | USB HID | A5? (check) |
| WushiController | 1 | USB HID | none? (check) |
| YeelightController | 1 | other (network/serial/OS) | A7 (serial) |
| ZETKeyboardController | 1 | USB HID | A5? (check) |
