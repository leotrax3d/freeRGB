# Plugin-System (Entwurf, nicht implementiert)

Vorbild SignalRGB [B]: JS-Module mit Metadaten-Exports und `Initialize/Render/Shutdown`.

Plan für Signal-RS:
1. **Deklarativ** (TOML): VID/PID/Interface, LED-Layout (Name, Position, Wire-Index),
   Verweis auf einen eingebauten Protokolladapter + Parameter (z. B. Bragi-Schema).
   Neue Bragi-Tastaturen ohne Codeänderung.
2. **Native Protokoll-Plugins** als separate Prozesse, die über die lokale API Frames
   empfangen und Geräte melden → Fehlerisolation, keine ABI-Probleme.
3. Versionierung über `api_version` im Manifest; Plugins ohne Signatur nur mit
   Nutzerfreigabe aktivierbar.
