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

A mock daemon for development, without hardware access:

```bash
cargo run -p freergb-daemon -- --no-hardware --virtual-device
```

For a Windows production build next to the daemon/launcher, run
`./scripts/build-desktop.ps1` from the repository root. This embeds the frontend;
no development server is needed. The installer build invokes this script too.
Requirements: Node.js 22+, stable Rust/MSVC build tools and WebView2 runtime.
Linux build dependencies and installation commands are in `docs/INSTALL.md`.

`src-tauri` is its own Cargo workspace so Tauri's dependencies never enter the
main workspace build. "Start service" looks for `freergbd` next to the app or in
`FREERGB_DAEMON`. Installed service starts use the adjacent launcher with
`--daemon-only`, preserving the optional provider configuration. Windows login
startup also uses that mode. `freergb-launcher --classic` opens the fallback;
the default launcher target is Tauri. Keep daemon, launcher and both GUI binaries
in the same installation directory to enable all cross-application actions.

Motherboard sensors, channel annotations, protected fan sessions/calibration and
keyboard reactions use the current main daemon API. Hotkeys, application profiles,
sensor history/warnings and updates remain available through Classic (open it
from Settings). Global shortcuts and application profiles need Classic running.
See `docs/INTEGRATION_CHECKLIST.md` for validation evidence and pending checks.

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
