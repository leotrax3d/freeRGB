# Protocols

Labels: **[C]** confirmed on hardware, **[P]** probable (statically evidenced),
**[U]** unknown.

## Corsair Bragi

Modern Corsair peripherals (keyboards, mice, dongles; also the base of the iCUE LINK hub).

### 1. Devices and interfaces
* K70 CORE RGB `1b1c:1bfd`, interface 1, usage page `0xFF42`, usage `0x0001` [C].
  Interface 2 (`0xFF42/0x0002`) carries notifications/input according to the plugin [P].
* Report lengths from the report descriptor: **64 bytes in/out, no report IDs** [C].
  The hidapi write buffer is therefore 65 bytes with a leading `0x00`.

### 2. Message format
Request (after report ID `0x00`): `[target, cmd, args…]`, `target = 0x08 | child_index`
(0 = directly attached device) [C for 0x08].
Response: `[x, cmd, err, data…]` (hidapi on Windows strips the report ID) [C].
`err = 0` → OK.

| cmd | Name | Arguments | Status |
|---|---|---|---|
| `0x12` | Ping | – | [C] |
| `0x02` | GetProperty | `prop:u16le` → value in `data[0..3]` (24-bit LE) | [C] |
| `0x01` | SetProperty | `prop:u16le, value:u24le` | [C] for `mode` only |
| `0x0D` | OpenEndpoint | `handle, endpoint` | [C] |
| `0x05` | CloseHandle | `0x01, handle` | [C] |
| `0x09` | CheckHandle | `handle, 0x00` → `err=3` ⇒ closed | [C] |
| `0x06` | WriteEndpoint | `handle, len:u32le, data…` | [C] |
| `0x07` | StreamEndpoint | `handle, data…` (continuation) | [C] |
| `0x08` | ReadEndpoint | `handle` | [P], not implemented |
| `0x15` | ConfirmChange | (Commander Core) | [P], not implemented |

Error codes [P]: 1 invalid value, 2 packet too long, 3 failed/unsupported,
5 property unsupported, 6 handle not open, 9 read-only.

### 3. Properties
| ID | Meaning | Read on K70 CORE |
|---|---|---|
| `0x03` | mode 1=hardware, 2=software | 1 / 2 [C] |
| `0x11` | VID | `0x1B1C` [C] |
| `0x12` | PID | `0x1BFD` [C] |
| `0x13` | firmware `a.b.c` = bytes 0,1,2 | `1.23.77` [C] |
| `0x41` | layout 1=ANSI, 2=ISO | 2 [C] |
| `0x01` | polling rate (4 = 1000 Hz) | 4 [C] |
| `0x02` | HW brightness 0–1000, **stored in flash according to the plugin** | never written |
| `0x36` | wireless child bitmask (dongles) | [P] |

**Safety rule:** writes only via the allowlist (`Property::is_writable`, currently only
`mode`). SignalRGB sets the flash brightness to 1000 on every start – freeRGB
deliberately does not.

### 4. RGB data
Handles: 0 Lighting, 1 Background, 2 Auxiliary. Endpoints: `0x01` Lighting (planar
`RRR…GGG…BBB`), `0x22` LightingController (`[0x12, 0x00]` + `RGB` per LED).
The K70 CORE accepts `0x22` [C]. LED index = HID usage ID of the key, modifiers
differ (L-Ctrl 105, L-Shift 106, L-Alt 107, L-Win 108, R-Ctrl 109, R-Shift 110,
R-Alt 111, Fn 122) [C for L-Ctrl, L-Shift, L-Win, Fn, R-Ctrl, ISO keys 50/100, Esc].
Buffer: 123 LED slots × 3 + 2 = 371 bytes → 1 write + 6 stream packets.
The K70 CORE requires `target=0x08` in stream packets too and acknowledges each [C]
(without acknowledgement the plugin uses `target=child_index`).

### 5. Sequence (implemented, `freergb-protocols::corsair_bragi`)
connect: ping → read VID/PID/FW/mode → check PID against model → close open handles →
mode software → probe endpoint `0x22` by opening/closing it on the Background handle →
open Lighting handle. Frame: WriteEndpoint + StreamEndpoint(s); on a protocol error the
handle is reopened once. disconnect: optional final colour, close handle, mode hardware.

### 6. Limits / open questions
* Frame rate: 21.9 FPS measured (debug build, acknowledged stream packets) [C].
  Cause and optimisation open [U].
* Wireless children (dongle, `target 0x09…`) not tested [U].
* Other Bragi models: tables missing; new models only with their own validation.

### 7. Reproducible tests
See `TESTING.md` ("Hardware tests – K70 CORE").

## Corsair iCUE LINK System Hub (`1b1c:0c3f`) – read-only analysis

Implementation: `freergb-protocols::corsair_bragi::link_hub`, CLI `freergb link-probe [--raw]`.
The hub drives fans and the AIO pump, so freeRGB only reads from it so far.

### Interface [C]
IF0, usage page `0xFF42`, usage `0x0001`; 512-byte input/output reports, no report IDs.

### Framing [C]
Request `[0x00 report id, 0x00, target, cmd, args…]`; the hub is target `0x01`
(the plugin sets the `0x08` bit only for lighting writes [P]).
Response (hidapi, report ID stripped) `[0x00, 0x00, cmd, err, data…]`.

Raw exchange 2026-10-18 (hardware mode):
```
> 00 00 01 12            ping            < 00 00 12 00 02
> 00 00 01 02 11 00      get VID         < 00 00 02 00 1c 1b
> 00 00 01 02 12 00      get PID         < 00 00 02 00 3f 0c
> 00 00 01 02 13 00      get firmware    < 00 00 02 00 04 01 90 02   → 4.1.144
> 00 00 01 02 03 00      get mode        < 00 00 02 00 01            → hardware
> 00 00 01 09 01 00      check handle 1  < 00 00 09 03               → closed
> 00 00 01 0d 01 36      open EP 0x36    < 00 00 0d 03               → error 3
```
Opening the child-list (`0x36`), fan-RPM (`0x17`) and temperature (`0x21`) endpoints
fails with error 3 while the hub is in **hardware mode** [C]. The plugin always
switches to software mode before reading them [P].

### Child list (endpoint `0x36`) [C, from a logged packet]
SignalRGB logged one raw child-list response on this machine. Payload after the error
byte: `[0x21, 0x00, slot_count, slot…]`. Every slot is either 8 zero bytes (empty) or
`[0, 0, type, subtype, 0, 0, 0x05, id_len, id[id_len] (ASCII)]`. The logged packet
decodes completely with this rule (15 slots, nothing left over):

| Slot | Type/sub | Model (plugin table) | LEDs |
|---|---|---|---|
| 0–2 | 15/0 | iCUE LINK RX Fan | 8 each |
| 3 | 17/2 | LINK Titan 360 RX (AIO) | 20 |
| 4 | 6/0 | LINK LCD | – |
| 5–11 | empty | | |
| 12, 14 | 27/0 | unknown to the plugin | ? |
| 13 | 2/0 | LX Fan per table [P] | 18 |

The plugin finds slots by scanning for a byte pattern, cuts both type-27 IDs down to
the same value (`000000C0`) and has no entry for type 27. This probably makes it
re-enumerate on every frame, which matches the ~100 MB of repeated log lines [P].


### Sensors and live read (2026-10-18) [C]
`freergb link-probe --software-window` (approved by the user): mode 1 → 2 for three
endpoint reads (~0.1 s), then back to 1; read back by the probe and by an independent
second run. Child list read live matches the logged packet slot for slot.

* Payload of fan RPM (`0x17`): data type `0x25 0x00` (the plugin assumes `0x06`), count 26,
  entries `(status, lo, hi)`; status `0` = valid, `1` = absent.
  Values: idx 1–3 = 728/729/726 rpm, idx 4 = 1698 rpm, idx 13 = 546, idx 14 = 877.
* Payload of temperatures (`0x21`): data type `0x10 0x00`, count 52, same entry layout,
  0.1 °C. Values: idx 0 = 33.1, idx 4 = 33.4, idx 13 = 24.2, idx 15 = 22.9 °C.
* Mapping sensor index → child slot is probably `slot + 1` (RX fans in slots 0–2 →
  idx 1–3, Titan AIO in slot 3 → idx 4 = pump rpm / coolant temperature) [P].
* Error 3 on `OpenEndpoint` in hardware mode; in software mode all three open [C].

### Lighting (2026-10-18) [C for RX fans, Titan pump head, LCD ring]
Implementation: `link_hub::{known_lighting_prefix, encode_lighting, lighting_packets,
LinkHubProbe::lighting_test}`, CLI `freergb link-light-test --seconds N` (user-approved).

* Requires software mode. Sequence: mode → software, open Lighting handle 0 on endpoint
  `0x22` (target `0x01`), `WriteEndpoint` with target `0x09` + u32 length (acknowledged),
  then `StreamEndpoint` packets with target `0x01` (not acknowledged). Afterwards close
  handle 0, mode → hardware, read back.
* Payload `[0x12, 0x00]` + RGB per LED, children in chain order. The test sent only the
  known prefix (RX 8 + RX 8 + RX 8 + Titan 20 + LCD 24 = 68 LEDs, 206 bytes); the hub
  accepted the shorter buffer without error for 96 frames.
* User observation: fan 1 red, fan 2 green, fan 3 blue exactly as sent → order and
  8-LED segments of the RX fans [C]. Second run: Titan pump head white and LCD ring
  magenta as sent → 20-LED pump segment and 24-LED LCD ring segment in this order [C].
* Fan and pump RPM stayed stable in software mode without any fan command
  (fans 722–755 rpm, pump 1675–1708 rpm vs. 726–734 / 1693 before) [C, 10 s window].
  The RPM watchdog (abort below 50 %) never triggered. Hardware mode verified afterwards.

### Daemon integration [C]
`LinkHubDevice` (`link_hub_device.rs`) implements `RgbDevice`; `freergbd --enable-link-hub`
opts in (off by default). Layout: one row per lit segment (RX, RX, RX, Titan, LCD = 68 LEDs).
`connect` only from hardware mode → software mode → child list → RPM baseline → open
lighting handle. `RpmWatchdog` (every 2 s; alarm after two consecutive samples that are
missing or < 50 % of the start value on a sensor that spun > 300 rpm) returns the hub to
hardware mode and blocks reconnects until the daemon restarts. Shutdown restores
hardware mode (verified).

### Next steps
Find out whether type 27 has LEDs (it limits the known prefix) and light the LX fan behind it. Fan speeds stay under hardware control;
writing fan or pump speeds is out of scope without explicit approval.

## Corsair Vengeance DDR5 (SMBus) – identified only
`i2c_scan_results.json` [C]: addresses `0x18` and `0x1A` on "AMD PIIX4 Bus 0x0b00",
mapped to `Corsair_Dominator_Ram.js`. SMBus writes to RAM are **risky** (SPD EEPROM
nearby) → only with explicit approval and a dedicated kernel/driver path; deferred.
