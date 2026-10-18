# Plugin system (design, not implemented)

Reference, SignalRGB [confirmed]: device plugins are JS modules exporting metadata
(`Name`, `VendorId`, `ProductId`, `LedPositions`, …) plus `Initialize/Render/Shutdown`.

Plan for freeRGB:
1. **Declarative** (TOML): VID/PID/interface, LED layout (name, position, wire index),
   reference to a built-in protocol adapter with parameters (e.g. Bragi scheme).
   New Bragi keyboards then need no code change.
2. **Native protocol plugins** as separate processes that register devices and receive
   frames through the local API → fault isolation, no ABI issues.
3. Versioning via `api_version` in the manifest; unsigned plugins only with user consent.
