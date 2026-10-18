# freeRGB

Standalone RGB lighting control written in Rust for Windows and Linux, built as an
independent replacement for SignalRGB.

**Status:** core library, effects, virtual device, a Corsair Bragi adapter validated on
real hardware (K70 CORE), the background service `freergbd` and its local JSON-RPC API.
GUI and plugin system are next – see `docs/ROADMAP.md`.

```
cargo test --workspace
cargo run -p freergb-cli -- discover              # list HID devices + matching adapters
cargo run -p freergb-cli -- bragi probe           # read-only device identification
cargo run -p freergb-daemon --bin freergbd        # start the service
cargo run -p freergb-cli -- ctl color.set '{"color":"#00ff00"}'
cargo run -p freergb-cli -- ctl daemon.shutdown   # devices return to hardware mode
```

Quit SignalRGB before driving hardware (both programs would fight over the device).
Documentation lives in `docs/`; start with `docs/HANDOFF.md`.
