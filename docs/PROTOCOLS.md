# Protokolle

Kennzeichnung: **[B]** bestätigt an Hardware, **[W]** wahrscheinlich (statisch belegt),
**[U]** unbekannt.

## Corsair Bragi

Moderne Corsair-Peripherie (Tastaturen, Mäuse, Dongles; auch Basis des iCUE-LINK-Hubs).

### 1. Geräte und Schnittstellen
* K70 CORE RGB `1b1c:1bfd`, Interface 1, Usage Page `0xFF42`, Usage `0x0001` [B].
  Interface 2 (`0xFF42/0x0002`) dient laut Plugin Benachrichtigungen/Eingaben [W].
* Report-Längen aus Report-Descriptor: **64 Byte In/Out, keine Report-IDs** [B].
  hidapi-Schreibpuffer daher 65 Byte mit führendem `0x00`.

### 2. Nachrichtenformat
Anfrage (nach Report-ID `0x00`): `[target, cmd, args…]`, `target = 0x08 | kindindex`
(0 = direkt angeschlossenes Gerät) [B für 0x08].
Antwort: `[x, cmd, err, data…]` (hidapi unter Windows liefert ohne Report-ID) [B].
`err = 0` → OK.

| cmd | Name | Argumente | Status |
|---|---|---|---|
| `0x12` | Ping | – | [B] |
| `0x02` | GetProperty | `prop:u16le` → Wert in `data[0..3]` (24 bit LE) | [B] |
| `0x01` | SetProperty | `prop:u16le, wert:u24le` | [B] nur für `mode` |
| `0x0D` | OpenEndpoint | `handle, endpoint` | [B] |
| `0x05` | CloseHandle | `0x01, handle` | [B] |
| `0x09` | CheckHandle | `handle, 0x00` → `err=3` ⇒ geschlossen | [B] |
| `0x06` | WriteEndpoint | `handle, len:u32le, data…` | [B] |
| `0x07` | StreamEndpoint | `handle, data…` (Fortsetzung) | [B] |
| `0x08` | ReadEndpoint | `handle` | [W], nicht implementiert |
| `0x15` | ConfirmChange | (Commander Core) | [W], nicht implementiert |

Fehlercodes [W]: 1 ungültiger Wert, 2 Paket zu lang, 3 fehlgeschlagen/nicht unterstützt,
5 Property nicht unterstützt, 6 Handle nicht offen, 9 schreibgeschützt.

### 3. Properties
| ID | Bedeutung | Gelesen an K70 CORE |
|---|---|---|
| `0x03` | Modus 1=Hardware, 2=Software | 1 bzw. 2 [B] |
| `0x11` | VID | `0x1B1C` [B] |
| `0x12` | PID | `0x1BFD` [B] |
| `0x13` | Firmware `a.b.c` = Bytes 0,1,2 | `1.23.77` [B] |
| `0x41` | Layout 1=ANSI, 2=ISO | 2 [B] |
| `0x01` | Polling-Rate (4 = 1000 Hz) | 4 [B] |
| `0x02` | HW-Helligkeit 0–1000, **laut Plugin im Flash gespeichert** | nicht geschrieben |
| `0x36` | Funk-Kindgeräte-Bitmaske (Dongles) | [W] |

**Sicherheitsregel:** Schreibzugriffe nur auf die Allowlist (`Property::is_writable`,
derzeit nur `mode`). SignalRGB setzt bei jedem Start die Flash-Helligkeit auf 1000 –
freeRGB tut das bewusst nicht.

### 4. RGB-Daten
Handles: 0 Lighting, 1 Background, 2 Auxiliary. Endpoints: `0x01` Lighting (planar
`RRR…GGG…BBB`), `0x22` LightingController (`[0x12, 0x00]` + `RGB` je LED).
Die K70 CORE akzeptiert `0x22` [B]. LED-Index = HID-Usage-ID der Taste, Modifier
abweichend (L-Ctrl 105, L-Shift 106, L-Alt 107, L-Win 108, R-Ctrl 109, R-Shift 110,
R-Alt 111, Fn 122) [B für L-Ctrl, L-Shift, L-Win, Fn, R-Ctrl, ISO-Tasten 50/100, Esc].
Puffer: 123 LED-Slots × 3 + 2 = 371 Byte → 1 Write- + 6 Stream-Pakete.
K70 CORE verlangt `target=0x08` auch in Stream-Paketen und quittiert jedes [B]
(ohne Quittung laut Plugin nur `target=kindindex`).

### 5. Ablauf (implementiert, `freergb-protocols::corsair_bragi`)
connect: Ping → VID/PID/FW/Mode lesen → PID gegen Modell prüfen → offene Handles
schließen → Modus Software → Endpoint `0x22` auf Background-Handle testweise öffnen/
schließen → Lighting-Handle öffnen. Frame: WriteEndpoint + StreamEndpoint(s); bei
Protokollfehler einmal Handle neu öffnen. disconnect: optional Endfarbe, Handle
schließen, Modus Hardware.

### 6. Grenzen / offene Fragen
* Bildrate: 21,9 FPS gemessen (Debug-Build, quittierte Stream-Pakete) [B]. Ursache und
  Optimierung offen [U].
* Funk-Kindgeräte (Dongle, `target 0x09…`) nicht getestet [U].
* Weitere Bragi-Modelle: Tabellen fehlen; neue Modelle nur mit eigener Validierung.

### 7. Reproduzierbare Tests
Siehe `docs/TESTING.md` („Hardwaretests K70 CORE“).

## Corsair iCUE LINK System Hub (`1b1c:0c3f`) – nur identifiziert
Laufzeitbelege (SignalRGB-Log, Thread `1b1c:0c3f:…`) [B]: Kindgeräte
3 × „RX Fan“ (Typ 15), „AIO LINK Titan 360 RX“ (Typ 17, Cooler-Typ 2), LCD (Typ 6),
außerdem Typen 2 und 27. Eindeutige IDs u. a. `0000C763`, `0000C769`, `0000382A`, `00019B75`.
Protokoll: Bragi-Basis mit eigenem Plugin `Corsair_ICUE_Link_Hub.js` [W] – noch nicht analysiert.

## Corsair Vengeance DDR5 (SMBus) – nur identifiziert
`i2c_scan_results.json` [B]: Adressen `0x18` und `0x1A` auf „AMD PIIX4 Bus 0x0b00“,
zugeordnet `Corsair_Dominator_Ram.js`. SMBus-Schreibzugriffe auf RAM gelten als
**riskant** (SPD-EEPROM in der Nähe) → nur mit ausdrücklicher Zustimmung und eigenem
Kernel-/Treiberzugang; zurückgestellt.
