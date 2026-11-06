# Instant Mouse provenance and audit

Upstream: OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/InstantMouseController`, GPL-2.0-or-later headers in every
controller source file.

| Upstream path or device | Audit | freeRGB mapping / rationale |
|---|---|---|
| `InstantMouseControllerDetect.cpp:17-32` — open matching HID path | ported | Host supplies the matching `Hid` handle to `A::initialize`; `table::device` retains the complete VID/PID/interface/usage match. |
| `InstantMouseController.cpp:16-27` — controller construction and close | ported | `A::initialize` establishes the module lifecycle and `A::shutdown` releases module state. The host, which owns `Hid`, closes the handle. |
| `RGBController_InstantMouse.cpp:168-184` — one `Mouse` zone and one LED | ported | `A::initialize` creates one `Mouse` LED in a 1×1 layout. |
| `RGBController_InstantMouse.cpp:186-199` — direct/per-zone/per-LED update | ported | `A::frame` accepts exactly one colour and calls `report::frames` once, yielding the complete six-feature-report direct sequence. |
| `InstantMouseController.cpp:57-80` — direct colour packet and all DPI slots | ported | `report::frames`; byte sequence test cites the upstream lines. |
| `RGBController_InstantMouse.cpp:39-47`, `201-216`; `InstantMouseController.cpp:83-115` — effect mode selection | not ported | freeRGB's module API exposes direct frames, not OpenRGB's effect-mode controls. No device is claimed to support those controls. |
| Advanced GTA 250 (`30fa:1030`) | ported | Exact detector and complete direct lifecycle above. |
| Anko KM43243952 (`30fa:1440`) | ported | Exact detector and complete direct lifecycle above. |
| Anko KM43277483 (`30fa:1540`) | ported | Exact detector and complete direct lifecycle above. |
| AntEsports GM600 (`30fa:1040`) | ported | Exact detector and complete direct lifecycle above; GM600-only non-direct modes are not exposed. |

The module intentionally reports all four devices as supported only for the
ported direct-frame lifecycle. It does not synthesize mode packets or claim
OpenRGB effect/profile functionality.
