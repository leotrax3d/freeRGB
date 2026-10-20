# GUI modernization: analysis and concept

Status: proposal (branch `feature/tauri-gui`). Nothing here changes the daemon, the
local API or the legacy egui GUI yet.

Goal: a second, modern desktop GUI (Tauri 2 + React + TypeScript + Tailwind +
shadcn/ui) next to the existing egui GUI, which stays as the "Classic" client for
low-resource PCs and as a fallback.

---

## 1. Inventory

### 1.1 Process model today

```
freergb-launcher ──(spawns if daemon.version fails)──▶ freergbd   (render + device workers, owns hardware)
       └──────────(always spawns)──────────────────▶ freergb-gui (egui, glow)
freergb-gui ── freergb-client ──JSON-RPC 2.0, one line per request──▶ \\.\pipe\freergb | $XDG_RUNTIME_DIR/freergb.sock
```

* The daemon is fully independent of the GUI. Closing the GUI never stops it; only
  `daemon.shutdown[_if]` does (installer/uninstaller use `shutdown_if`).
* `freergb-client::call` is blocking, opens a fresh connection per call, 3 s deadline,
  256 KiB request / 16 MiB reply caps. No persistent connection, no events.
* The GUI holds no device or protocol code — this boundary must be kept.

### 1.2 How the egui GUI talks to the daemon

One worker thread (`main.rs::worker`):

* every 1 s (or right after a command) it calls `devices.list`, `diagnostics.get`,
  `effects.get`, then optionally `profiles.list`, `plugins.list`,
  `custom_effects.list`, `cooling.curves.list`, `macros.list`, `macros.status`
  → **9 connections per second**, whole state replaced each time.
* UI commands go through an mpsc queue; setter bursts (`brightness.set`, `color.set`,
  `effects.set`, `devices.set_region`, `devices.set_brightness`) are coalesced per
  method + device after the last non-setter barrier (`push_command`, unit-tested).
* GUI-only pseudo methods: `gui.apply_custom_effect` (get + set),
  `gui.load_app_profile`, `gui.export_diagnostics`.
* Repaint only when new state arrives (`request_repaint_after(1s)`).

### 1.3 Feature inventory (must reach parity)

| Area | Feature | Where the logic lives |
|---|---|---|
| Shell | Daemon connected/unreachable, measured FPS, last error, conflict banner (other RGB software running), sensor-warning banner, "app profiles active – pause", "hotkeys active" | GUI (display) |
| Devices | List with status (connected/disconnected/faulted), LED count, frames/dropped, last error, per-device brightness | Daemon |
| Plugins | List, start/stop allowlisted IDs, status/error | Daemon |
| Lighting | Effect picker (12 effects), per-effect parameters, global brightness | Daemon |
| Lighting | Layers editor (1–8 layers, blend, opacity, optional region, reorder) | Daemon validates, GUI edits |
| Lighting | Custom effects save / apply / delete | Daemon |
| Layout | Canvas editor: drag/resize device regions | Daemon (`devices.set_region`) |
| Profiles | Save / load / delete (disabled with `--no-state`) | Daemon |
| Profiles | **Application profiles** (Windows foreground process → profile, debounced, opt-in per session) | **GUI process** (`app_profiles.rs`, `application-profiles.json`) |
| Cooling | Curve editor, save/delete, preview duty (simulated only) | Daemon + `freergb-control` model |
| Macros | Visual step editor + advanced JSON, save/delete/run/cancel, status | Daemon |
| Macros | **Global hotkeys** (session opt-in, Ctrl+Shift+F11 cancel) | **GUI process** (`hotkeys.rs`, `hotkeys.json`, `global-hotkey`) |
| Monitoring | Sensor list, **bounded histories with stale gaps** | **GUI process** (`sensor_history.rs`) |
| Monitoring | **Sensor warning rules** (debounce, hysteresis, acknowledgement) | **GUI process** (`alerts.rs`, `sensor-alerts.json`) |
| Monitoring | Diagnostics export (redacted report → file) | Daemon builds, GUI saves |
| Updates | Check / download / install GitHub releases, opt-in periodic checks | **GUI process** (`freergb-update`, `automatic-updates.json`) |
| Cooling | Custom Celsius source graphs (input, offset, mix, time average) | Daemon (`cooling.sensors.get/save/sources/values`, stored in `control.json`, see `CUSTOM_SENSORS.md`) |

The bold rows are the main integration problem: several features are not daemon
features but logic that runs inside the egui process. Custom sensors were moved
into the daemon on `main` (commit `cac3a86`) before this work started, and the
Classic GUI no longer offers a daemon shutdown action — both GUIs now consume
the same readings through JSON-RPC.

### 1.4 Data models used by the GUI

* `EffectSpec` – serde tagged enum (`{"type":"breathing","color":{"r":..,"g":..,"b":..},"period_ms":3000}`),
  variants: solid, rainbow_wave, breathing, gradient, color_cycle, radial_rainbow,
  scanner, twinkle, audio_pulse, audio_spectrum, screen_ambience, layers.
* `DeviceSummary` – `{info:{id,name,vendor,kind,usb_vid,usb_pid,serial,firmware}, status, brightness, region, leds, frames, dropped, last_error, sensors[]}`;
  `kind` ∈ Keyboard, Mouse, Headset, LightingController, FanHub, Cooler, Memory, Virtual, Other;
  `status` = `"Connected" | "Disconnected" | {"Faulted": "…"}`.
* `Diagnostics` – uptime, ticks, target/measured FPS, effect, brightness, devices,
  conflicts, audio, screen.
* Validation ranges are documented in `LOCAL_API.md` (speed −100..100, period
  1..86 400 000 ms, …) and enforced by the daemon.

### 1.5 Configuration files (`%APPDATA%\freeRGB`, `$XDG_CONFIG_HOME/freergb`)

Daemon: `state.json`, `profiles/*.json`, custom effects, `control.json` (curves,
macros, custom sensor graph `version: 1`).
GUI: `application-profiles.json`, `hotkeys.json`, `sensor-alerts.json`,
`automatic-updates.json`, diagnostic reports. All JSON, all must stay compatible.

### 1.6 Build and packaging

* Cargo workspace, CI on Windows/Linux (`ci.yml`), package workflow, Inno Setup
  installer (`packaging/windows/freergb.iss`) shipping `freergbd`, `freergb-gui`,
  `freergb-launcher`, CLI; Start-menu/desktop/autostart point at the launcher.
* Toolchain on this machine: Rust 1.99, Node 24, npm 11, WebView2 runtime present.

### 1.7 Brand and existing web code

* `assets/refined.svg|png` (primary logo): lowercase "free" in `#ECECEC` above
  three rounded tiles **R `#EF4444` · G `#10A37F` · B `#3B82F6`** with white
  letters on black, subtle noise texture. Variants: `monochrome`, `light`,
  `light_no-gradient`. The logo is exactly the spec palette — the tile motif is the
  visual identity to build on.
* `website/` is a separate marketing site (React 18, Tailwind 3, shadcn/Radix,
  lucide, Inter + Manrope from Google Fonts, neutral zinc-like greys). The desktop
  app does not depend on it and does not share its build; it may share brand
  assets. Differences on purpose: the desktop app bundles fonts locally (no network)
  and uses the spec palette instead of the website's neutral HSL tokens.
* `skills/shadcn/` contains shadcn usage guidance for agents; the desktop app's
  component work follows it where it does not conflict with this document.

---

## 2. Technical challenges

1. **GUI-resident logic.** App profiles, hotkeys, alerts, sensor history and updates
   run in the GUI process and are interleaved with egui code.
   The new GUI must not re-implement them in TypeScript (that would be a second,
   diverging implementation). → Extract the egui-independent parts into a shared
   Rust crate and run them in the Tauri backend (see §3.4).
2. **Two GUIs at once.** Both could register the same global hotkeys and both could
   switch profiles on focus changes. → A per-user single-owner lock for "desktop
   automation" (hotkeys + app profiles + update scheduler); the second GUI shows
   these features as "active in another freeRGB window".
3. **No push channel.** The API has no event subscription (listed as planned).
   Polling stays, but moves into Rust, is diffed, and paused while the window is
   hidden/minimized (§3.3). An `events.subscribe` API is a later, optional,
   backwards-compatible addition — not required for this project.
4. **The webview cannot open a named pipe.** All daemon access goes through Tauri
   commands in Rust that reuse `freergb-client` unchanged.
5. **Resource cost.** WebView2 adds roughly 80–150 MB RAM and some start-up time
   versus egui (~30–50 MB). This is inherent and is exactly why Classic stays.
   We keep the web side small, but we should not promise egui-level memory.
6. **Workspace/CI weight.** Tauri pulls a large dependency tree and needs Node and
   (on Linux) webkit2gtk. Adding it to the main workspace would slow every
   `cargo build --workspace`/`clippy` run and change CI requirements. → Separate
   Cargo workspace for the desktop app (§3.1).
7. **Window close semantics.** Tauri exits when the last window closes (no tray, no
   hidden window). The app must never call `daemon.shutdown`.
8. **Ongoing work on `main`.** `main.rs`/`pages.rs` keep changing. Extracting
   shared logic out of `freergb-gui` touches the same files, so that step is
   coordinated with `main` (§6, M5). `main` agreed to keep new GUI-side logic
   egui-free and to announce API/config additions in commit messages.
9. **Type drift.** TS types mirror Rust serde types. → Contract fixtures generated
   from Rust (`EffectSpec::default_for` for every name, a sample `DeviceSummary`)
   checked by a TS test, instead of a code generator dependency.

---

## 3. Architecture

### 3.1 Repository layout

```
apps/desktop/                 # new GUI, own Cargo workspace + npm package
  package.json  vite.config.ts  index.html  tsconfig.json  components.json
  src/
    main.tsx  App.tsx
    app/                      # shell: sidebar, status bar, banners, command palette, view registry
    features/
      lighting/ devices/ layout/ profiles/ cooling/ macros/ monitoring/ settings/
        (each: View.tsx, components, feature hooks)
    components/ui/            # shadcn/ui sources, restyled to our tokens
    components/               # shared composites (ColorPicker, ParamField, Sparkline, …)
    lib/api/                  # invoke wrapper, event subscription, types.ts, fixtures test
    lib/store/                # zustand store split in slices
    lib/mock/                 # MOCK transport for browser-only design work (clearly labelled)
    styles/tokens.css  styles/globals.css
  src-tauri/
    Cargo.toml                # path deps: freergb-client, freergb-effects, freergb-core, freergb-control
    tauri.conf.json  capabilities/default.json
    src/main.rs  rpc.rs  poller.rs  commands/…
crates/freergb-gui/           # unchanged: Classic GUI
```

Why a separate workspace: the main workspace, CI matrix and `Cargo.lock` stay
untouched; Tauri dependencies never enter `cargo build --workspace`. Shared crates
are consumed via `path` dependencies. A dedicated CI job builds the desktop app.

### 3.2 Layers

```
React views ──▶ zustand store (slices, selectors) ──▶ lib/api (invoke / listen)
                                                          │ Tauri IPC
src-tauri:  commands ──▶ command queue + coalescing ──▶ freergb-client ──▶ freergbd
            poller   ──▶ diff ──▶ emit("state://<slice>")
            desktop services (later): app profiles, hotkeys, alerts, history, updates
```

* **Presentation**: React components, no API calls inside components except through
  feature hooks.
* **State**: one zustand store with slices (`connection`, `devices`, `effect`,
  `profiles`, `customEffects`, `plugins`, `control`, `diagnostics`). Components
  subscribe with narrow selectors → a sensor update re-renders only the sensor rows.
* **Communication**: `lib/api` is the only module importing `@tauri-apps/api`.
  In a plain browser (`npm run dev` without Tauri) it switches to `lib/mock`.
* **Types**: `lib/api/types.ts`, hand-written from serde types, guarded by fixtures.

### 3.3 Communication concept

* **Commands** – `rpc(method, params)` Tauri command with a method allowlist that
  mirrors `LOCAL_API.md` (no `daemon.shutdown*`). The Rust side reuses the same
  queue + `push_command` coalescing as egui (ported, same unit tests), so slider
  drags never flood the pipe. The frontend additionally throttles drags to one
  send per animation frame and always sends the final value on release.
* **State** – the poller performs the same queries as egui but:
  * diffs each slice against the previous value and emits only changed slices;
  * runs right after any command (fast feedback) and otherwise every 1 s;
  * queries view-specific data only while that view is open (the frontend tells
    the backend which slices it needs, e.g. `cooling.sensors.values` only on
    Cooling/Monitoring) — fewer daemon calls than the egui worker today;
  * slows to ≥ 5 s when the window is minimized/hidden, resumes on focus;
  * reports connection loss once (`state://connection`), then backs off.
* **Optimistic UI** – controls keep a local "pending" value until the daemon
  confirms (same model as egui's `device_brightness`/`device_regions`), so polling
  never makes a slider jump back.
* **Errors** – RPC errors (`-326xx`, `-3200x`) map to inline field errors or a
  toast; transport errors switch the app into the "service unavailable" state.
* **Starting the service** – if the daemon is unreachable the app offers "Start
  service", spawning `freergbd` next to the executable exactly like the launcher.
* **Protocol changes required now: none.**

### 3.4 Desktop services (GUI-resident logic)

Step 1 (M5): move the egui-independent parts of `alerts`, `app_profiles`,
`hotkeys`, `sensor_history` and update scheduling into a new crate
`crates/freergb-desktop` (rules, validation, debounce/hysteresis state machines,
JSON file formats, platform hooks). The egui GUI keeps only its UI code and calls
the crate → identical behavior and file formats in both GUIs, tests move along.

Step 2: the Tauri backend runs these services and exposes them as commands/events.

Optional later: moving app profiles/hotkeys/alerts into the daemon would make them
work without any GUI open. That changes the daemon's scope (input/focus access in
the service process) and is deliberately **not** part of this plan.

### 3.5 Tauri configuration

* Tauri 2, no shell/fs/http plugins. Possible small plugins:
  `single-instance` (focus existing window), `window-state` (size/position).
* Strict CSP, all assets local (fonts bundled, no CDN, no telemetry, no network
  except the existing GitHub update check in Rust).
* Capabilities: only our own commands and window events.
* Native window decorations with dark theme in the first iteration (keeps Windows
  snap layouts and accessibility). A custom title bar is a later, separate decision.
* Close = process exit; daemon is not touched.

### 3.6 Stack choices and deviations

| Choice | Reason |
|---|---|
| Vite | Standard Tauri frontend tooling, fast HMR, small output |
| React 19 + TS strict | as requested |
| Tailwind v4 + CSS variables | tokens defined once in `tokens.css`, Tailwind theme reads them |
| shadcn/ui (Radix primitives) | accessible base for Dialog, Popover, Select, Slider, Switch, Tooltip, DropdownMenu, Tabs, Toast (sonner); copied in and restyled |
| lucide-react | icons, tree-shaken imports only |
| zustand (~1 KB) | selector-based subscriptions; less boilerplate than context, avoids re-render cascades |
| **No router** | 8 top-level views; a typed view registry + `React.lazy` is enough |
| **No Motion library (for now)** | CSS transitions cover hover/press/enter; Motion only if layout animations turn out to be needed |
| **No chart library** | sparklines and the fan-curve editor are small custom SVG components |
| **Own colour picker** | HSV area + hue strip + hex input + recent swatches, ~200 lines; fits the design better than a generic one |

---

## 4. Design concept

### 4.1 Idea

"The light is the content." The interface stays monochrome and quiet — black,
graphite surfaces, soft text — so the only saturated colours on screen are the
user's own lighting colours and a few semantic accents. Calm like a modern chat
app, but organized around devices and light rather than conversations.

The logo's three rounded R/G/B tiles become the recurring shape language: colour
swatches, effect previews and device colour chips are rounded "tiles" (radius
scale below), and the R/G/B trio appears in the sidebar mark and the
"service unavailable"/empty states — nowhere as decoration on ordinary controls.

### 4.2 Navigation and layout

```
┌───────────┬───────────────────────────────────────────────┬──────────────┐
│ freeRGB   │  Lighting                         ⌘K  ● 60 fps│              │
│           │ ┌───────────────────────────────────────────┐ │  Inspector   │
│ ◉ Lighting│ │  Live canvas strip / device layout preview │ │  (parameters │
│ ▢ Devices │ └───────────────────────────────────────────┘ │   of the     │
│ ▦ Layout  │  Effects                                      │   selected   │
│ ☰ Profiles│  [Solid][Wave][Breath][Gradient][Cycle] …     │   effect,    │
│ ✇ Cooling │  Custom effects                               │   brightness │
│ ⌨ Macros  │  [My layer mix] [+ New]                       │   device)    │
│ ∿ Monitor │                                               │              │
│           │                                               │              │
│ ⚙ Settings│                                               │              │
│ ● Service │                                               │              │
└───────────┴───────────────────────────────────────────────┴──────────────┘
```

* **Sidebar** (collapsible to icons): Lighting (home), Devices, Layout, Profiles,
  Cooling, Macros, Monitoring; Settings (updates, plugins, diagnostics, about) at
  the bottom with the service status dot.
* **Content + inspector**: lists/tiles in the middle, editing in a right inspector
  panel. Selecting something never navigates away, it opens the inspector.
* **Banners** directly under the header for conflicts ("SignalRGB is running —
  freeRGB leaves your hardware alone"), sensor warnings and automation state.
* **Command palette** (Ctrl+K): jump to views, apply effects, load profiles.
* **Global brightness** lives in the header of Lighting and in the palette.
* **Window**: min 960×600, inspector collapses below 1200 px.

### 4.3 Key views

* **Lighting** – effect tiles with a static preview swatch derived from the
  effect's parameters (gradient/colours; no reimplementation of effects). The
  selected tile animates subtly only while the window is visible. Inspector shows
  the effect's parameters from a declarative schema (field type, range, unit,
  step) matching the daemon's validation ranges. Layers open as a vertical stack
  with drag-to-reorder, blend and opacity per layer.
* **Devices** – cards grouped by kind (keyboards, controllers, coolers…): icon,
  name, vendor, status dot, LED count, inline brightness. Faulted devices show the
  error and the frames/dropped counters; detail in the inspector (firmware, IDs on
  demand, sensors).
* **Layout** – canvas with device rectangles in their own colours, snap to 5 %
  grid, keyboard nudging (arrow keys), numeric fields in the inspector.
* **Profiles** – saved profiles as rows with load/delete; application profile rules
  below (process → profile), with the explicit session opt-in toggle.
* **Cooling** – SVG curve editor (draggable points, temperature/duty axes),
  "simulated preview" clearly labelled because hardware duty writes are disabled.
* **Macros** – step list editor, run/cancel, status; hotkey bindings with a
  shortcut recorder; advanced JSON in a code-styled textarea.
* **Monitoring** – sensors with value, age and sparkline (stale gaps visible),
  warning rules with acknowledgement, diagnostics export.

### 4.4 Design tokens

Base palette from the spec, extended into semantic tokens:

| Token | Value | Use |
|---|---|---|
| `--bg` | `#000000` | window background, sidebar |
| `--surface` | `#212121` | content panels, cards |
| `--surface-raised` | `#303030` | hover, popovers, inputs, selected rows |
| `--surface-sunken` | `#171717` | canvas, code areas, track of sliders |
| `--border` | `rgb(255 255 255 / 0.08)` | hairlines |
| `--border-strong` | `rgb(255 255 255 / 0.16)` | input borders, focus-adjacent |
| `--text` | `#ECECEC` | primary text |
| `--text-muted` | `#B4B4B4` | secondary text |
| `--text-subtle` | `#8F8F8F` | captions, units, placeholders |
| `--text-disabled` | `#5D5D5D` | disabled |
| `--accent` | `#10A37F` | active/selected state, success, "connected" |
| `--danger` | `#EF4444` (text: `#F87171`) | errors, destructive |
| `--warning` | `#F59E0B` | conflicts, sensor warnings |
| `--info` | `#3B82F6` (text: `#60A5FA`) | info, links, focus ring |
| `--creative` | `#A855F7` (text: `#C084FC`) | custom effects / layers |

Contrast notes (WCAG AA, 4.5:1 for small text):
* White text on `#10A37F` is only ~3.2:1 → **primary buttons are light
  (`#ECECEC` with black text)**; green marks state, not button labels. Green text on
  `#212121` is ~5:1 and fine.
* Red/blue/violet base values are ~4.0–4.4:1 on `#212121`, so text uses the lighter
  variants listed above; base values are for fills, icons and dots.

Scales:
* **Spacing**: 4 px grid (4, 8, 12, 16, 20, 24, 32, 48).
* **Radii**: 6 (chips, small controls), 10 (buttons, inputs), 14 (cards),
  20 (panels, dialogs), full (swatches, status dots, pills).
* **Type**: Inter (variable, bundled locally); 12 caption · 13 small · 14 body ·
  16 title · 20 section · 28 page title; weights 400/500/600; `tabular-nums` for
  all numbers (FPS, RPM, °C, %).
* **Elevation**: by surface colour, not shadows; one soft shadow only for
  popovers/dialogs.
* **Motion**: 120 ms hover/press, 180 ms enter, 240 ms panels,
  `cubic-bezier(0.2, 0, 0, 1)`; `prefers-reduced-motion` disables non-essential
  motion; no infinite animations except the selected effect preview while visible.

### 4.5 States and accessibility

* **Loading**: skeletons only if a request takes > 150 ms; otherwise nothing.
* **Empty**: short explanation + one action ("No devices found — check that the
  device is connected, or start the demo device").
* **Service unavailable**: full-view state with retry, "Start service", and a hint
  to the Classic GUI/CLI.
* **Errors**: inline next to the control for validation errors, toast for actions,
  persistent banner for daemon-level conditions.
* **Focus**: visible 2 px `--info` ring on every interactive element; full keyboard
  navigation (sidebar with arrow keys, Ctrl+K, Esc closes inspector/dialogs);
  Radix primitives provide ARIA roles; sliders accept arrow/PageUp/Home/End;
  colour is never the only signal (status dots carry text/tooltip).

### 4.6 Component set

AppShell, Sidebar/NavItem, Header, StatusPill, Banner, Surface/Card, Inspector,
EmptyState, Skeleton, Toast; DeviceCard, DeviceKindIcon, StatusDot;
EffectTile, EffectPreview, ParamField (Slider, NumberInput, ColorField, Select,
Switch) driven by the effect schema, ColorPicker, BrightnessControl, LayerStack;
LayoutCanvas; CurveEditor; Sparkline; MacroStepList, ShortcutRecorder;
CommandPalette, ConfirmDialog.

---

## 5. Performance budget

| Metric | Target |
|---|---|
| First paint | < 1 s on a mid-range PC (warm WebView2) |
| Idle CPU, window visible | < 0.5 % (1 Hz polling, no animation) |
| Idle CPU, minimized | ~0 % (polling ≥ 5 s, no rendering) |
| Initial JS (gzip) | < 250 KB; heavy views lazy-loaded |
| RAM (app + WebView2) | aim < 150 MB; Classic remains the low-memory option |
| Daemon load | ≤ today's egui load (same queries, fewer when hidden) |

Means: diffed slice events from Rust, narrow store selectors, `React.memo` on
list rows, no JS timers for polling, CSS-only transitions on `transform`/`opacity`,
rAF-throttled drag sends, lazy views, no runtime CSS-in-JS.

---

## 6. Migration plan

| Step | Content | Touches existing code? |
|---|---|---|
| **M0** | Scaffold `apps/desktop` (Tauri, Vite, Tailwind tokens, shadcn base), shell + sidebar + connection state via `daemon.version`/`diagnostics.get`, mock transport | No |
| **M1** | Read-only: devices, diagnostics, current effect, plugins, conflicts banner | No |
| **M2** | Lighting control: effects + parameter schema, colour picker, global and per-device brightness, profiles, custom effects; command coalescing ported + tested | No |
| **M3** | Layout canvas editor, layers editor | No |
| **M4** | Cooling curves + preview, custom sensor graph editor and live values (`cooling.sensors.*`), macros (editor, run/cancel, status), monitoring view (current values), diagnostics export | No |
| **M5** | Extract `crates/freergb-desktop` from `freergb-gui` (alerts, app profiles, hotkeys, history, updates) → both GUIs use it; automation owner lock | **Yes** – coordinated with `main` |
| **M6** | Packaging: installer ships both GUIs; launcher reads `gui.json` (`"desktop"` / `"classic"`), Start-menu entry "freeRGB Classic"; CI job for the desktop app (Node + Tauri build, frontend tests) | Yes – installer/launcher, small |
| **M7** | Parity checklist (table §1.3) passed → desktop GUI becomes default; Classic stays supported | Config default only |

Progress (branch `feature/tauri-gui`):
* **M0 + M1 done** – scaffold, shell, connection/service state, devices (read +
  per-device brightness), conflicts banner.
* **M2 done** – effect parameters (schema-driven, ranges checked against daemon
  bounds by tests), HSV colour picker, global brightness, custom effects
  (save/apply/delete), profiles (save/load/delete, `--no-state` handled).
  Application profiles stay in Classic until M5.
* **M3 done** – layout canvas (drag, corner resize, 5 % snapping with Alt to
  disable, keyboard nudging, numeric sliders) and layer editor (1–8 layers,
  top-first list, effect/blend/opacity, per-layer parameters, optional region
  drawn on a mini canvas, reorder/remove).
* **M4 done** – Cooling (curve editor with draggable points, limits, daemon-
  simulated preview; custom temperature source graph editor with live values),
  Macros (step editor for all step types, repeat, save/run/stop, status),
  Monitoring (service stats, device sensors with staleness, custom sources,
  diagnostics export to the same folder as Classic), Settings (plugins
  start/stop, about). Client-side rules mirror `FanCurve::validate`,
  `MacroDefinition::validate`, `chord_plan` and sensor node names; checked
  against the real daemon.
* Remaining for parity (M5): application profiles, global hotkeys, sensor
  history graphs, warning rules, updates.

End-to-end check (2026-10-20, debug build, `freergbd --no-hardware
--virtual-device` with an isolated `APPDATA`): connection, effect/brightness
writes, error propagation, `daemon.shutdown` blocked, 60-write burst coalesced
to 5 daemon calls with the last value applied, profiles/custom effects stored,
curves/macros/sensor graph validation identical to the daemon, macro run,
diagnostics export, window close exits the app while the daemon keeps running.
Measured: backend 38 MB and ~0.2 % CPU idle; WebView2 ~270 MB in the debug/dev
setup (release numbers below).

### Memory tuning (release build, 2026-10-20)

Task Manager value (private working set of the app plus all WebView2 processes),
Lighting/Layout view open, virtual device:

| Configuration | Processes | Task Manager | Private bytes |
|---|---|---|---|
| Tauri defaults | 7 | 138 MB | 251 MB |
| + software rendering, GPU stage in browser process | 6 | 94 MB | 168 MB |
| + network service in process | 5 | 80–83 MB | 124 MB |
| + background networking, component updates, sync off (shipped) | 5 | **73–78 MB** | 117–121 MB |
| shipped, window minimized 30 s | 5 | **7–8 MB** | – |
| shipped, after restore | 5 | 24 MB (grows again on use) | – |

* Flags live in `tauri.conf.json` (`additionalBrowserArgs` replaces wry's
  defaults, so those are repeated). Software rendering showed no measurable CPU
  difference against GPU rendering for a 60 fps repaint at 1200×760; very large
  or 4K windows may cost more CPU and should be re-checked on such displays.
* Minimizing switches WebView2's `MemoryUsageTargetLevel` to Low
  (`src-tauri/src/memory.rs`, the crate's only `unsafe` block); restoring
  shows the first frame after ~120 ms.
* No effect: renderer process limit, V8 lite mode, low-end device mode,
  disabling further browser features. The JS heap is ~3 MB; the remaining cost
  is the WebView2 runtime itself.

Rules during the transition:
* Classic must build, run and pass its tests after every step.
* No daemon/API changes are needed for M0–M4. Any later API addition (e.g. events)
  must be additive and versioned via `daemon.version.api_version`.
* New desktop-only UI preferences go into `desktop-ui.json`; all shared files keep
  their existing format.
* Testing: Vitest for store/schema/fixtures, Rust unit tests for coalescing and
  diffing, manual runs against `freergbd --no-hardware --virtual-device`.

---

## 7. Open decisions for the maintainer

1. Separate Cargo workspace for `apps/desktop` (recommended) vs. joining the main
   workspace.
2. Timing of M5 (shared desktop crate) relative to the custom-sensor work on `main`.
3. Whether desktop automation (app profiles, hotkeys, alerts) should eventually move
   into the daemon so it runs without an open GUI.
4. Native title bar (recommended for now) vs. custom title bar.
