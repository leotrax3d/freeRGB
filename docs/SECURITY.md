# Sicherheit

* Unbekannte Geräte werden nur gelesen/inventarisiert, nie beschrieben.
* Schreibzugriffe nur über validierte Adapter; Property-Writes per Allowlist.
* Keine Firmware-, Flash-, Treiber- oder SMBus-Schreibzugriffe ohne ausdrückliche Zustimmung.
* `disconnect` gibt Geräte immer an den Hardware-Modus zurück, auch nach Fehlern (CLI).
* `unsafe_code = "forbid"` im gesamten Workspace.
* Lokale API (geplant): nur IPC (Named Pipe/Unix-Socket mit Benutzer-ACL), kein Netzwerkport.
* Keine Übernahme personenbezogener Daten aus SignalRGB-Logs (z. B. Machine-ID).
