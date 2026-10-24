# Mainboard RGB controllers (USB) – reference list

This list shows which mainboard RGB/ARGB controllers can in principle be driven
over the board's **internal USB** connection, i.e. without SMBus/PawnIO. Fans
and strips are plugged into the board's 4-pin 12 V RGB or 3-pin 5 V ARGB
headers as usual; freeRGB talks to the controller chip behind those headers.

Source: device IDs registered by OpenRGB detectors at commit
`7182807a288a9e8107a84fbebc44239677f1ea24`
(<https://github.com/CalcProgrammer1/OpenRGB/tree/7182807a288a9e8107a84fbebc44239677f1ea24/Controllers>).
Only identifiers (facts) are listed here; no OpenRGB code is included.
"OpenRGB" means OpenRGB detects the device; it is not freeRGB hardware evidence.

## freeRGB status

| Status | Meaning |
| --- | --- |
| **Supported (experimental)** | Implemented in freeRGB, see `MAINBOARD_RGB.md` |
| **Planned** | Covered by `docs/promts/multi-fan-control-and-mainboard-rgb.md` |
| **Not planned** | SMBus-only controllers (need a driver) |

## How to find your controller

Windows: Device Manager → *Human Interface Devices* → device properties →
*Details* → *Hardware IDs* (`VID_xxxx&PID_yyyy`).
Linux: `lsusb`. MSI boards are identified by their **MS-xxxx** product code,
printed on the board and in the manual.

## ASUS Aura USB (VID `0b05`)

| PID | Type | freeRGB |
| --- | --- | --- |
| `19af` | Aura Motherboard (usage page `ff72`, usage `00a1`) | Supported (experimental) |
| `18f3` | Aura Motherboard | Planned |
| `1939` | Aura Motherboard | Planned |
| `1aa6` | Aura Motherboard | Planned |
| `1bed` | Aura Motherboard | Planned |
| `1867`, `1872`, `18a3`, `18a5` | Aura Addressable header controller | Planned |

Covers most ASUS boards roughly from the 400/500 series on. Older ASUS boards
with Aura over SMBus are not listed.

## MSI Mystic Light (VID `1462`, PID = MS product code)

Detected by OpenRGB (tested set), freeRGB status **Planned**:

```
MS-1720 MS-7B12 MS-7B16 MS-7B17 MS-7B18 MS-7B50 MS-7B85 MS-7B92 MS-7B93 MS-7C02 MS-7C34 MS-7C35
MS-7C36 MS-7C37 MS-7C56 MS-7C59 MS-7C60 MS-7C67 MS-7C71 MS-7C73 MS-7C75 MS-7C76 MS-7C77 MS-7C79
MS-7C80 MS-7C81 MS-7C82 MS-7C83 MS-7C84 MS-7C86 MS-7C87 MS-7C90 MS-7C91 MS-7C92 MS-7C94 MS-7C95
MS-7C98 MS-7D03 MS-7D04 MS-7D06 MS-7D07 MS-7D08 MS-7D09 MS-7D13 MS-7D14 MS-7D15 MS-7D17 MS-7D18
MS-7D19 MS-7D20 MS-7D25 MS-7D27 MS-7D28 MS-7D29 MS-7D30 MS-7D31 MS-7D32 MS-7D33 MS-7D36 MS-7D37
MS-7D38 MS-7D40 MS-7D41 MS-7D42 MS-7D43 MS-7D46 MS-7D50 MS-7D51 MS-7D52 MS-7D53 MS-7D54 MS-7D59
MS-7D67 MS-7D69 MS-7D70 MS-7D73 MS-7D74 MS-7D75 MS-7D76 MS-7D77 MS-7D78 MS-7D86 MS-7D88 MS-7D89
MS-7D90 MS-7D91 MS-7D93 MS-7D96 MS-7D97 MS-7D98 MS-7D99 MS-7E01 MS-7E03 MS-7E06 MS-7E07 MS-7E09
MS-7E10 MS-7E20 MS-7E30 MS-7E32 MS-7E34 MS-7E59 MS-7E66 MS-7E70 MS-7E76 MS-7E80 MS-7E81 MS-7E86
MS-B926
```

Common PID `0db0:0076` covers a further set of 185-byte boards, including X870
(usage page `ff00`). OpenRGB lists these additional codes only behind
`ENABLE_UNTESTED_MYSTIC_LIGHT` (disabled by default): `MS-3EA4 MS-4459 MS-7B10 MS-7B94 MS-7B96 MS-7C42 MS-7C70 MS-7C85 MS-7C88 MS-7C89 MS-7C96 MS-7C99 MS-905D`.

Older MSI boards without a USB Mystic Light controller are not covered.

## Gigabyte RGB Fusion 2 USB (ITE IT5702/IT8297)

| VID:PID | freeRGB |
| --- | --- |
| `048d:8297` | Planned |
| `048d:5702` | Planned |
| `048d:5711` | Planned |
| `048d:8950` | Planned |
| `0414:a100` | Planned |

Detected by chip, not by board name: any Gigabyte board with one of these chips
(roughly X470/Z390 and newer) is covered. Header names such as `D_LED1`,
`ARGB_V2_1` come from the controller's own table. Older boards using RGB Fusion
over SMBus are not covered.

## ASRock Polychrome USB (VID `26ce`)

| PID | Type | freeRGB |
| --- | --- | --- |
| `01a2` | Polychrome USB motherboard | Planned |
| `01a6` | DeskMini addressable LED strip | Planned |

Many ASRock boards use Polychrome over SMBus instead; those are not covered.

## Not covered (SMBus / driver required)

ASUS Aura SMBus (older boards), ASRock Polychrome SMBus, Gigabyte RGB Fusion
(SMBus), older MSI boards, Biostar and OEM boards (Dell, HP, Lenovo).
