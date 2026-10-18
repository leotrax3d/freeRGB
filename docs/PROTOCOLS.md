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

## Corsair iCUE LINK System Hub (`1b1c:0c3f`) – identified only
Runtime evidence (SignalRGB log, thread `1b1c:0c3f:…`) [C]: children
3 × "RX Fan" (type 15), "AIO LINK Titan 360 RX" (type 17, cooler type 2), LCD (type 6),
plus types 2 and 27. Unique IDs include `0000C763`, `0000C769`, `0000382A`, `00019B75`.
Protocol: Bragi base with its own plugin `Corsair_ICUE_Link_Hub.js` [P] – not analysed yet.

## Corsair Vengeance DDR5 (SMBus) – identified only
`i2c_scan_results.json` [C]: addresses `0x18` and `0x1A` on "AMD PIIX4 Bus 0x0b00",
mapped to `Corsair_Dominator_Ram.js`. SMBus writes to RAM are **risky** (SPD EEPROM
nearby) → only with explicit approval and a dedicated kernel/driver path; deferred.
