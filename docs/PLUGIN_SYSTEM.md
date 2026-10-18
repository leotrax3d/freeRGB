# Plugin system

Reference, SignalRGB [confirmed]: device plugins are JavaScript modules exporting metadata
(`Name`, `VendorId`, `ProductId`, `LedPositions`, …) plus `Initialize/Render/Shutdown`.

freeRGB splits this into two stages:

| Stage | What | Status |
|---|---|---|
| 1 | **Declarative device descriptions** (TOML): identity, adapter, adapter parameters, LED layout | ✅ implemented |
| 2 | **Out-of-process protocol plugins** for new protocols, talking to `freergbd` over the local API | ⏳ planned |

## Stage 1: device descriptions

A description tells freeRGB *which built-in protocol adapter* drives a device and *what its
LEDs look like*. Adding another keyboard that speaks a supported protocol needs no Rust code.

Built-in descriptions live in
[`crates/freergb-protocols/devices/`](../crates/freergb-protocols/devices/) and are compiled
into the binary. Example (shortened) – the CORSAIR K70 CORE:

```toml
[device]
name = "CORSAIR K70 CORE RGB"
vendor = "Corsair"
kind = "keyboard"            # keyboard | mouse | headset | lighting_controller | fan_hub | cooler | memory | other
adapter = "corsair-bragi"    # must be an implemented adapter
support = "validated"        # experimental | validated
vendor_id = 0x1B1C
product_ids = [0x1BFD, 0x1BFF]
usage_page = 0xFF42          # HID interface the adapter talks to
usage = 0x0001

[bragi]                      # adapter parameters
stream_ack = true            # continuation packets are acknowledged

[layout]
width = 21
height = 6
leds = [
  { name = "Esc", x = 0, y = 0, wire = 41 },   # x/y: grid cell, wire: index in the device's colour buffer
  { name = "F1",  x = 2, y = 0, wire = 58 },
  # …
]
```

Validation (on load and with `freergb validate-device <file>`): known adapter, kind and
support level; at least one product id; adapter parameters present; every LED inside the
grid; unique LED names and grid cells; unique wire indices; unknown keys are rejected.

### User descriptions (opt-in)

```bash
freergb validate-device my-keyboard.toml      # no hardware access
freergbd --user-devices ~/freergb-devices     # load every *.toml in that directory
```

* User descriptions are **always treated as experimental**, whatever `support` says.
* They are **off by default**; `freergbd` only reads them with `--user-devices`, and logs a
  warning for every description it uses.
* Built-in descriptions win if both match the same interface.
* A description can only point a device at an existing adapter. It cannot add new commands,
  so it cannot make freeRGB send anything the adapter would not send to a validated device.
  Still: only describe devices whose protocol you know, and test with short, reversible
  colour tests first (see `REVERSE_ENGINEERING.md`).

Currently supported adapters for descriptions: `corsair-bragi`.

## Stage 2: protocol plugins (planned)

Native protocol plugins as separate processes that register devices and receive frames
through the local API → fault isolation, no Rust ABI issues, any language. Versioning via an
`api_version` in the plugin manifest; unsigned plugins only with user consent.
