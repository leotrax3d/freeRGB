# Decisions

1. **No SignalRGB plugin code is reused.** Its licence is unknown; only interface facts
   (command bytes, LED indices) are used, with their source documented.
2. **hidapi with native backends** (`windows-native`, `linux-native`) – no C dependency.
3. **Report lengths come from the report descriptor** (own parser), not hard-coded values.
4. **Property write allowlist** – only `mode` is writable; flash brightness is never written.
5. **Effects are pure functions of time** – deterministic tests.
6. **Local API = JSON-RPC 2.0 over named pipe / Unix socket**, line-delimited. No TCP
   port by default; readable with any language, easy to test without sockets.
7. **Render loop on a dedicated thread, API on tokio.** HID calls block; tokio is only
   used where async I/O helps (IPC).
8. **Engine behind one mutex** for now. Simple and correct; an API call waits at most
   one frame. Revisit if the GUI needs higher-rate state streaming.
9. **Plugin system (planned):** declarative device descriptions (TOML) for layouts and
   known protocol variants; complex protocols as out-of-process plugins talking to the
   API instead of Rust dylibs (no stable Rust ABI). See `PLUGIN_SYSTEM.md`.
10. **Project name freeRGB, repo language English.**
