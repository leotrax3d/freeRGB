# Signal-RS

Eigenständige RGB-Steuerung in Rust (Windows/Linux) als unabhängiger Ersatz für SignalRGB.

**Stand:** Kern, virtuelles Gerät, Effekte und ein an echter Hardware validierter
Corsair-Bragi-Adapter (K70 CORE). Daemon, API und GUI folgen – siehe `docs/ROADMAP.md`.

```
cargo test --workspace
cargo run -p signal-cli -- discover
cargo run -p signal-cli -- bragi probe
cargo run -p signal-cli -- bragi effect rainbow --seconds 10
```

Vor Hardwaretests SignalRGB beenden. Dokumentation: `docs/` (Einstieg: `docs/HANDOFF.md`).
