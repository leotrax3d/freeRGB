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
11. **FIFO HID admission (2026-10-19):** retain process-wide serialization but
    allocate tickets before waiting on a condition variable. A worker cannot
    reacquire ahead of already queued workers. Guards advance the queue on drop,
    including unwind. This prevents overtaking, not slow HID calls or OS scheduling
    delays; hardware throughput improvements require measurement. No new dependency.
12. **Process plugins use private versioned stdio, not the shared API for frames.**
    A startup manifest allowlist feeds normal device workers; the local API manages
    start/stop/status. This avoids Rust ABI compatibility and API session ownership
    complexity. Existing latest-frame slots and reconnect backoff are reused.
    Tokio process pipes bound both reads and writes; faulted children are terminated.
    Native plugin execution is explicit trust, not sandboxing. One hardware plugin
    excludes built-in adapters until a shared cross-process transport exists.
13. **Validate saved state as strictly as API input.** Corrupt or semantically invalid
    startup state falls back with a warning; invalid profiles fail without mutating
    current state. Layered effects validate every contained effect's parameters.
14. **Broad motherboard fan support like FanControl (2026-10-22, project owner).**
    freeRGB controls every fan header LibreHardwareMonitor can drive, not only
    chips with audited register readback, accepting lower assurance than the
    previous policy. Support is tiered: *verified* (register readback, today's
    Nuvoton separate-mode path), *LHM-supported* (behavioural safeguards:
    per-channel opt-in after a guided RPM-response test, runtime RPM plausibility
    and fail-safe return to firmware), *read-only*. Pumps and unidentified
    channels are never reduced. Plan: `COOLING_BROAD_SUPPORT_PLAN.md`.
15. **One cooling evaluator in the Rust daemon (2026-11-04).** Every controller
    type runs in `freergb-control` inside `freergbd`; boot-time operation is
    the same daemon started headless (`--cooling-service`) with an independent
    watchdog, not a second evaluator in the protected C# service. The C# service
    keeps the native boundary. Its `BackgroundCooling` simple-curve mirror stays
    only until the protected service can authorize boot channels for a
    service-launched daemon (owner review + hardware, prompt 07). Cooling
    profiles never contain electrical channel bindings. See `COOLING_SERVICE.md`.
16. **Host-owned platform identity (2026-11-06).** HID API 1 gains optional
    `platform` labels on identify/initialize with default SDK delegation, so
    shipped modules and older hosts keep working. Read-only Windows registry
    and Linux sysfs discovery stays in the host, cached once per process and
    injectable in simulations. Only bounded board vendor/name and system
    product labels cross the broker; no serials, UUIDs, firmware blobs or raw
    handles. Exact model gating remains a module responsibility and grants no
    hardware write authority. This enables future DMI-gated detection without
    adding family ports or claiming hardware acceptance. See
    [HARDWARE_MODULES.md](HARDWARE_MODULES.md#read-only-platform-identity-api-1-additive-extension).


17. **Read-only HID identity extension v1 (2026-11-06).** Keep the API-1 envelope
    and legacy startup unchanged; opt in through a signed, versioned package
    probe policy. Before initialization, broker only explicitly allowed serial/
    product strings and feature report IDs/maximum lengths through the same
    daemon-owned handle and global HID gate. Reject every probe write even if
    a child bypasses the SDK. Keep strings transient and out of diagnostics.
    Protocols needing output writes for identity (Logitech receiver HID++ name)
    remain outside this extension. Family ports are separate external work.
    See HARDWARE_MODULES.md, "Read-only identity probes".
18. **Host-driven HID module timing (2026-11-06).** Keep outer protocol v1
    compatible with old modules, and version an optional initialize-result
    timing extension independently. Device workers call maintenance even without
    frames; modules never spawn timer threads or receive device handles. Host
    intervals are 500 ms to 10 s, with no catch-up bursts. Brokered per-packet
    delays release the HID gate, consume the existing call/deadline budget,
    and are capped at 100 ms each / 1 s total per request. API 2 bus and API 3
    cooling scheduling remain unchanged. Simulated fixture tests establish the
    generic timing contract; external port authors own vendor packet and
    hardware acceptance. See HARDWARE_MODULES.md.
19. **Opt-in hardware effect modes (2026-11-06).** Keep HID API 1 and negotiate
    a separately versioned catalog in initialize output. Old modules retain the
    existing wire shape and default SDK hooks. Validate OpenRGB-compatible mode
    metadata and parameters in host and SDK; execute commands on the owning
    worker through the existing broker. Hardware animation suspends streamed
    frames until explicit direct mode. Saving is a separate confirmed operation;
    automatically persistent modes are refused. No selection persistence or
    reconnect replay. Original fixture and virtual-device tests only, no family
    ports or hardware acceptance. See HARDWARE_MODULES.md, hardware effect modes.
