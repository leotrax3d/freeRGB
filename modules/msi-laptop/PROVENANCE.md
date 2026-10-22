# MSI laptop provenance and audit

Upstream: OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/MSILaptopController`; all five files are GPL-2.0-or-later.

| Upstream path or device | Audit | Reason |
|---|---|---|
| `MSILaptopControllerDetect.cpp:290-333` | ported (inventory only) | Four detector registrations are generated. |
| `MSILaptopControllerDetect.cpp:29-288` model maps | not ported | Per-device keyboard matrices, LED offsets and lightbar maps are absent. |
| `MSILaptopController.cpp:21-31` lifecycle | simplified | Host ownership was represented, but device type and cleanup semantics were not. |
| `MSILaptopController.cpp:62-145` reports | not ported | Keyboard uses 525-byte feature chunks plus 65-byte commit; lightbar uses a separate feature report. |
| `RGBController_MSILaptop.cpp:53-133` zones | not ported | Model-specific matrix and lightbar zone layouts were replaced by one guessed LED. |
| MSI Laptop Keyboard `1038:1122` | not ported | Layout and complete report sequence missing. |
| MSI Laptop Lightbar `1038:1161` | not ported | Lightbar mapping and report missing. |
| MSI GE68HX Keyboard `1038:113a` | not ported | Layout and complete report sequence missing. |
| MSI GE68HX Lightbar `1038:114b` | not ported | Lightbar mapping and report missing. |

The unrelated generic packet builder was removed and all devices fail closed.
