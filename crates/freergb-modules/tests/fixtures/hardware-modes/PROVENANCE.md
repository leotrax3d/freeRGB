# Hardware-mode fixture provenance

This fixture is original freeRGB test code under MIT OR Apache-2.0, like its
containing crate. No OpenRGB functions, controller packets or family code were
ported. Existing GPL family COPYING/provenance files are unchanged.

| Reference | Commit | Use |
|---|---|---|
| OpenRGB `RGBController/RGBControllerInterface.h`, mode flags/directions/color types and mode fields | `bbc77470a7cba56af60989e54ba910d5ac66a3ac` | Data-contract correspondence documented in HARDWARE_MODULES.md; independently implemented validation, no copied functions |

The fixture executable is `src/bin/freergb-module-fixture.rs`, selected with
`fixture-mode` content `hardware-modes`. Synthetic reports are initialize
`[0,0xAB]` (echo required), set `[3,7,speed,direction,brightness,color_count,RGB...]`,
direct `[4]`, save `[5]`, streaming `[1,RGB...]`, shutdown `[2,red]`.
These bytes do not represent any real device protocol.

## Test status

Original fixture, simulated HID only. Unit tests cover catalog/selection
validation; real-child-process tests cover brokered exact bytes, volatile
selection, frame suppression, explicit save refusal/confirmation, direct resume,
legacy operation and reconnect reset. Daemon tests cover RPC and owning-worker
serialization without ticks. No hardware support or nonvolatile storage behavior
is claimed.

## Owner acceptance steps

1. A future family porter must audit every advertised mode against its pinned
   upstream controller, provide byte-exact protocol tests and preserve that
   module's GPL COPYING/provenance. Do not advertise automatic-save paths.
2. With owner-approved hardware access, list modes and verify native speed,
   direction, brightness and color-count bounds against the device.
3. Select a volatile mode, verify streamed frames do not replace it, then return
   explicitly to direct mode. Disconnect/reconnect must not replay the selection.
4. Verify selection and shutdown issue no save packet. Only after an explicit
   owner confirmation, save and power-cycle to assess physical persistence.
5. Record device/firmware, exact tested modes and results in that family module's
   PROVENANCE.md. The fixture cannot establish real hardware acceptance.
