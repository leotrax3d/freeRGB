# Entscheidungen

1. **Kein Plugin-Code aus SignalRGB** – Lizenz unbekannt; nur Schnittstellenfakten mit Quelle.
2. **hidapi mit nativen Backends** (`windows-native`, `linux-native`) – keine C-Abhängigkeit.
3. **Report-Längen aus dem Report-Descriptor** statt fester Werte (eigener Parser).
4. **Property-Allowlist** – nur `mode` schreibbar; Flash-Helligkeit wird nie geschrieben.
5. **Effekte als reine Zeitfunktionen** – deterministische Tests.
6. **Plugin-System (geplant):** deklarative Gerätebeschreibungen (TOML/JSON) für Layouts
   und bekannte Protokollvarianten; komplexe Protokolle als Out-of-Process-Plugins über
   die lokale API statt Rust-dylibs (keine stabile Rust-ABI). Details: `PLUGIN_SYSTEM.md`.
7. **Lokale API (geplant):** Named Pipe (Windows) / Unix-Socket (Linux), JSON-RPC,
   kein Netzwerk-Port per Default.
