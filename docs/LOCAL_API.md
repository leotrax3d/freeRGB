# Lokale API (Entwurf, nicht implementiert)

Transport: Windows Named Pipe `\\.\pipe\signal-rs` (ACL: aktueller Benutzer),
Linux `$XDG_RUNTIME_DIR/signal-rs.sock` (0600). Protokoll: JSON-RPC 2.0, zeilengetrennt.

Geplante Methoden: `devices.list`, `devices.status`, `effects.list`, `effects.set`
(`EffectSpec` aus `signal-effects`), `color.set`, `profiles.load|save`,
`layouts.get|set`, `diagnostics.get`, `plugins.list`, `frames.push` (Ambience/Plugins).
