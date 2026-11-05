# Plugin system

Downloadable, signed hardware modules (separate protocol, daemon-owned device
handles) are described in [HARDWARE_MODULES.md](HARDWARE_MODULES.md).

Reference, SignalRGB [confirmed]: device plugins are JavaScript modules exporting metadata
(`Name`, `VendorId`, `ProductId`, `LedPositions`, …) plus `Initialize/Render/Shutdown`.

freeRGB splits this into two stages:

| Stage | What | Status |
|---|---|---|
| 1 | **Declarative device descriptions** (TOML): identity, adapter, adapter parameters, LED layout | ✅ implemented |
| 2 | **Out-of-process device plugins**, versioned stdio protocol; managed through local API | ✅ foundation implemented; virtual reference tested, hardware plugin validation pending |

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

Keyboard LEDs may additionally declare `key_usage = N`, an explicit USB keyboard
page usage (4..115 or 224..231). Usages must be unique and may appear only on a
`kind = "keyboard"` description. It maps the key to that LED's **layout** index,
independently of `wire`. Omitted mappings remain unsupported, never guessed from
names/wire indices. Existing descriptions without this optional field remain
valid. The built-in K70 CORE mapping handles Corsair modifier remaps separately;
Fn and the unverified ABNT-specific key remain unmapped. See `KEYBOARD_REACTIVE.md`.

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

## Stage 2: process device plugins

`freergb-plugins` launches one explicitly trusted executable per manifest/device.
No shared Rust ABI, installation, automatic discovery or network access is needed.
The child receives initialized layout-order frames from the existing engine, including
global/device brightness and canvas placement. GUI/device diagnostics work as usual.

Build with `cargo build --workspace --release`, then from the repository root:

```powershell
./target/release/freergbd.exe --no-hardware --no-state --virtual-device --plugin examples/plugins/virtual-windows.json
./target/release/freergb.exe ctl plugins.list
./target/release/freergb.exe ctl plugins.stop '{"id":"example-strip"}'
./target/release/freergb.exe ctl plugins.start '{"id":"example-strip"}'
```

Linux uses `examples/plugins/virtual-linux.json` and executables without `.exe`.
The Linux example is supplied for CI/future validation; it has not been run locally.
The GUI's Plugins section exposes the same start/stop controls.

### Manifest and trust

See the complete examples under `examples/plugins/`. Required fields:
`api_version` (1), `id` (1–64 ASCII letters/digits/`-`/`_`), `name`, `executable`,
`access` (`virtual` or `hardware`), `layout`. Optional `args` defaults to `[]`;
`timeout_ms` defaults to 1000 and must be 50–5000. Layouts contain 1–4096 LEDs,
valid positions and unique names. Unknown manifest fields and duplicate plugin IDs
are rejected before any plugins start. At most 16 manifests are allowed.

Paths resolve relative to the manifest, are canonicalized, and must name a file.
Commands are launched directly without a shell or PATH lookup; Windows children
start without a console window. Supplying `--plugin PATH` is an explicit trust
decision: **native plugins run with the daemon user's privileges and are not
sandboxed**. They must not spawn descendants. The access field is a declaration,
not an OS permission boundary. Only use reviewed plugins. No signatures or download
manager are implemented. Replacing an executable changes the code used on restart.

`--no-hardware` leaves declared hardware plugins disabled and prevents API start.
At most one hardware plugin is allowed, and its configuration excludes built-in
hardware adapters for the entire daemon run. This prevents assuming the process-local
HID gate can serialize another process. SignalRGB/iCUE/OpenRGB conflict detection
also suspends hardware plugins; virtual plugins continue. No real hardware process
plugin is shipped or validated yet. A plugin must use only evidence-backed commands
and handle its own recovery; killing a failed child cannot restore physical hardware.

### Wire protocol version 1

UTF-8 JSON lines over child stdin/stdout; no unsolicited stdout/logging. Stderr is
discarded to avoid unbounded capture. One outstanding request per device, monotonic
request IDs, no notifications/batching. Each message including newline is at most
262144 bytes. Requests:

```json
{"api_version":1,"id":1,"method":"initialize","params":{"layout":{"width":1,"height":1,"leds":[{"name":"LED 1","pos":[0,0]}]}}}
{"api_version":1,"id":2,"method":"frame","params":{"colors":[{"r":255,"g":0,"b":0}]}}
{"api_version":1,"id":3,"method":"shutdown","params":{"final_color":null}}
```

Successful response to each: `{"api_version":1,"id":1,"result":true}` using the
request's ID. Failure: `{"api_version":1,"id":1,"error":"reason"}`.
Initialize must complete before frames; shutdown must restore/release the device
**before acknowledging**. The host then terminates/reaps the child. EOF must also
trigger the plugin's own cleanup. `final_color` is null or an RGB object.

The request deadline includes writing, flushing and reading, so a nonreading child
cannot fill a pipe indefinitely. OS process creation itself is outside that deadline.
Wrong IDs/versions, invalid JSON, oversized replies, EOF, error replies and timeout
fault the device and terminate the child. The engine retries with existing exponential
backoff (2–60 seconds). Stop removes the worker; start re-adds it on the next render
tick. Cleanup waits up to one second after requesting termination; OS failures and
uncooperative descendant processes are outside the isolation guarantee.

The lifecycle API only accepts IDs from the startup allowlist; it cannot introduce
an executable. Enabled state is session-local and is not saved to profiles.

### Reference and tests

`freergb-plugin-virtual` is a Rust reference implementation. Optional `--log PATH`
records request lines. `--fault` modes are test fixtures (hang, no-read, crash,
crash-frame, hang-shutdown, malformed, oversized, wrong-id, wrong-version).
`cargo test -p freergb-plugins` runs real child processes and tests lifecycle,
exact RGB values, reconnect, frame size checks, malformed manifests and fault handling.
`scripts/smoke-daemon.ps1 -WithPlugin` covers real Windows IPC and start/stop/restart.

Effect packages have a separate signed marketplace, WASM process sandbox and live
reload lifecycle; see [EFFECT_PLUGINS.md](EFFECT_PLUGINS.md). This does not change
the native device-plugin trust model described above. Hardware protocol validation,
brokered shared transport and native device-plugin installation/hot reload remain
future work.
