# Architektur

```
freergb-cli ──┐                       (später: freergbd Daemon, freergb-gui)
             ├─ freergb-protocols ── freergb-device-api ── freergb-core
             ├─ freergb-hid ───────── freergb-device-api
             └─ freergb-effects ───── freergb-core
```

* **freergb-core** – Rgb/Hsv, Farbkorrektur, LedLayout, Canvas, Frame. Keine I/O.
* **freergb-device-api** – `HidTransport` (rohe Reports), `RgbDevice` (identify,
  connect, push_frame, status, disconnect), `VirtualDevice`, `MockTransport`.
* **freergb-protocols** – Herstellerprotokolle als reine Paketfunktionen + Zustandsmaschine
  generisch über `HidTransport`; `registry` ordnet Interfaces Adaptern zu.
* **freergb-hid** – hidapi (native Backends Windows/Linux), Report-Descriptor-Parser.
* **freergb-effects** – Effekte als reine Funktionen der Zeit → deterministisch testbar.

Prinzipien: Transport, Protokoll und Gerätefunktion getrennt; unbekannte Geräte werden
nie beschrieben; Property-Schreibzugriffe nur per Allowlist; `unsafe` workspaceweit verboten.
Entscheidungen: `docs/DECISIONS.md`.
