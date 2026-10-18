# Devices

Inventory of the development system (Windows 11 Pro 26300), collected 2026-10-18 via
PnP and `freergb discover`. All entries confirmed.

| Device | VID:PID | RGB interface | SignalRGB plugin | freeRGB status |
|---|---|---|---|---|
| CORSAIR K70 CORE RGB (ISO, FW 1.23.77) | 1b1c:1bfd | IF1 `FF42:0001` | Modern_Corsair/Corsair_Bragi_Device.js | **Validated** (colour, per-key sample, mode switch, daemon) |
| CORSAIR iCUE LINK System Hub (FW 4.1.144) | 1b1c:0c3f | IF0 `FF42:0001` | Modern_Corsair/Corsair_ICUE_Link_Hub.js | Experimental, in daemon via `--enable-link-hub`: RX fans, pump head, LCD ring; RPM watchdog; 15-min soak passed |
| ↳ 3 × RX fan, Titan 360 RX AIO, LCD ring, LX fan (lit, confirmed), iCUE COMMANDER DUO (type 27, 2 entries, no LEDs) | (via hub) | – | – | Lighting confirmed for all LED children |
| CORSAIR iCUE LINK AIO LCD Screen Module | 1b1c:0c4e | `000C:0001` | Corsair/Corsair_ICUE_Link_LCD.js | Identified |
| Corsair Vengeance RGB DDR5 CMH32GX5M2B6000Z30 (2 modules) | SMBus 0x18, 0x1A | SMBus (PIIX4) | Corsair/Ram/Corsair_Dominator_Ram.js | Identified, deferred (risk) |
| ASUS AURA LED Controller (mainboard) | 0b05:19af | IF2 `FF72:00A1` | Asus/Motherboard Controllers | Identified |
| Razer Cobra (mouse) | 1532:00a3 | – | Razer/Razer_Modern_Mouse.js | Identified |
| Logitech USB Receiver | 046d:c548 | – | – | not RGB-relevant |
| unknown, no USB interface | 1b1c:1bf0 | keyboard/mouse collections | – | [unknown] probably a virtual HID device |
| GPUs: NVIDIA RTX 5060 Ti (Gigabyte), AMD RX 9060 XT, AMD iGPU | PCI | I2C/NvAPI | – | not investigated |

## Validation log
K70 CORE, 2026-10-18, SignalRGB stopped:
1. `bragi probe` (read-only): VID/PID/FW/layout/poll rate correct.
2. `bragi color #ff0000 --seconds 20`: user confirmed "fully red".
3. `bragi keys …`: Esc, L-Ctrl, L-Shift, L-Win, Fn, ISO `<`, ISO `#`, R-Ctrl each in its
   own colour – user confirmed "all correct" (also confirms RGB channel order).
4. After every test `mode = 1 (hardware)` was read back.
5. `bragi effect rainbow --fps 60`: 132 frames / 6 s = 21.9 FPS, no errors.
6. `freergbd --virtual-device`: K70 + virtual strip connected, `color.set` via API,
   measured 22 FPS, `daemon.shutdown` → mode 1 read back.

iCUE LINK hub, 2026-10-18, SignalRGB stopped, each step approved by the user:
1. `link-probe` (read-only): VID/PID, FW 4.1.144, mode 1; endpoints closed in hardware mode.
2. `link-probe --software-window`: 8 children, fan/pump RPM, temperatures; mode 1 verified.
3. `link-light-test --seconds 10`: fans 1–3 red/green/blue confirmed by the user; RPM stable;
   mode 1 verified twice.
4. Second `link-light-test --seconds 10`: pump head white and LCD ring magenta confirmed; mode 1 verified twice.
5. Soak test 1 (`link-light-test --seconds 900`): aborted after ~4.4 min by the first watchdog
   version on a single unreadable RPM sample (all measured values ≥ 719 rpm); hardware mode
   verified. Watchdog changed to "two bad samples in a row".
6. Soak test 2 (15 min): 8619 frames, 0 glitches, fans 838–1047 rpm, pump 1721–1946 rpm
   (slowly rising in software mode); hardware mode verified.
7. `freergbd --enable-link-hub`: hub (68 LEDs) and K70 CORE connected together, `color.set`
   via API, 528 frames each without errors at ~22 FPS; `daemon.shutdown` → both devices read
   back mode 1. Colour on the fans not visually confirmed in this run.
8. Type 27 identified as iCUE COMMANDER DUO (0 LED channels, per OpenLinkHub database; GPL-3.0, facts only). `link-light-test --seconds 15` with the LX fan as 6th segment (after the LCD ring, Commander DUO skipped): user confirmed the LX fan fully yellow.
9. Threaded engine (one worker per device), 2026-10-18: without a global HID gate the K70 CORE
   vendor interface hung after 6–55 s (write `WaitTimedOut`, needed re-plug) – with pipelined
   *and* with classic streaming. With all hidapi calls serialised: 2 min at target 60 FPS,
   hub 52.4 FPS, K70 19.1 FPS, no errors; both devices read back mode 1 after shutdown.
   Pipelined streaming stays experimental (`bragi --pipelined`), untested with the gate.
10. Pipelining retest with the HID gate (`freergbd --enable-link-hub`, 5 min, temporary
    `--experimental-pipelining` flag): K70 26 FPS for ~2.3 min, then "timed out waiting for a
    response" and the vendor interface hung again (re-plug needed). Hub stayed connected
    (28 FPS while the K70 streamed, 46 FPS after it dropped out); mode 1 verified after
    shutdown. **Conclusion: the K70 CORE needs per-packet acknowledgement.** The daemon flag
    was removed again; pipelining remains a research option in `freergb bragi --pipelined`.
11. 15-min daemon soak, classic streaming, serialised HID (no keepalive yet): hub 47–57 FPS
    for the full 15 min; K70 19 FPS for ~9.5 min, then the keyboard answered "device error 3
    (handle closed)", the reopen failed and the interface hung (write `WaitTimedOut`). While the
    K70 kept failing, its 2-s reconnect attempts (1 s write timeout each) held the HID gate
    and slowed the hub to ~38 FPS → exponential back-off added. After the user re-plugged
    the K70 the worker kept using the dead handle → unplugged devices are now dropped and
    re-added by the hot-plug scan.
12. 20-min daemon soak with keepalive (ping 50 s, mode check 5 s) and plugin pacing
    (`70fe64b`): K70 CORE 20.8–21.1 FPS and hub 57.4 FPS for the full 20 min, no errors, no
    "left software mode" event; both devices read back mode 1 after shutdown. Latency summaries
    (one per minute) flat over the whole run: frame avg 47.1–47.7 ms, max 182–191 ms;
    6–10 acknowledgements per minute take ~135 ms (periodic, not growing). **The ~10-minute
    dropout is fixed.**
13. Live test with the user watching (`freergbd --enable-link-hub`): `audio_spectrum` first
    reacted "only on the left" (bass) – band energies were the mean over all FFT bins and shared
    one gain reference, so treble bands stayed near zero. Fixed (peak bin per band, per-band
    auto gain floored at 5 % of the loudest band); user: "movement everywhere" on K70 and fans.
    `screen_ambience` (saturation 1.6): user confirmed keyboard and fans follow the screen.
    Daemon at ~59 FPS; both devices back in mode 1 after shutdown.
