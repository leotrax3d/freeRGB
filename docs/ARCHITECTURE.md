# Architektur

```
signal-cli ──┐                       (später: signal-rsd Daemon, signal-gui)
             ├─ signal-protocols ── signal-device-api ── signal-core
             ├─ signal-hid ───────── signal-device-api
             └─ signal-effects ───── signal-core
```

* **signal-core** – Rgb/Hsv, Farbkorrektur, LedLayout, Canvas, Frame. Keine I/O.
* **signal-device-api** – `HidTransport` (rohe Reports), `RgbDevice` (identify,
  connect, push_frame, status, disconnect), `VirtualDevice`, `MockTransport`.
* **signal-protocols** – Herstellerprotokolle als reine Paketfunktionen + Zustandsmaschine
  generisch über `HidTransport`; `registry` ordnet Interfaces Adaptern zu.
* **signal-hid** – hidapi (native Backends Windows/Linux), Report-Descriptor-Parser.
* **signal-effects** – Effekte als reine Funktionen der Zeit → deterministisch testbar.

Prinzipien: Transport, Protokoll und Gerätefunktion getrennt; unbekannte Geräte werden
nie beschrieben; Property-Schreibzugriffe nur per Allowlist; `unsafe` workspaceweit verboten.
Entscheidungen: `docs/DECISIONS.md`.
