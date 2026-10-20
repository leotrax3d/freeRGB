# freeRGB desktop (Tauri)

Modern desktop GUI for freeRGB. It talks to `freergbd` only through the local
JSON-RPC API, like freeRGB Classic (`crates/freergb-gui`), which stays available.
Concept and migration plan: [`docs/GUI_MODERNIZATION.md`](../../docs/GUI_MODERNIZATION.md).

## Development

```bash
npm install
npm run dev          # UI only, in a browser, with clearly labelled MOCK data
npm run tauri dev    # real desktop window against a running freergbd
npm test             # frontend tests (contract fixtures, logic)
```

Backend tests (from `src-tauri/`): `cargo test`. After changing effect serde
types, regenerate fixtures with `UPDATE_FIXTURES=1 cargo test --test fixtures`.

A safe daemon for development, without touching hardware:

```bash
cargo run -p freergb-daemon -- --no-hardware --virtual-device
```

`src-tauri` is its own Cargo workspace so Tauri's dependencies never enter the
main workspace build. "Start service" looks for `freergbd` next to the app or in
`FREERGB_DAEMON`.

## Structure

| Path | Purpose |
|---|---|
| `src-tauri/src/queue.rs` | Serialized, coalescing command queue; method allowlist (no shutdown) |
| `src-tauri/src/poller.rs` | Polls daemon state, emits only changed slices, slows down while hidden |
| `src/lib/api` | Transport (Tauri IPC or mock), types mirroring Rust serde, fixtures |
| `src/lib/store` | zustand store fed by slice events; throttled control writes |
| `src/styles/globals.css` | Design tokens (colour, type, radius, motion) |
| `src/app` | Shell, sidebar, view registry |
| `src/features/*` | One folder per view |
