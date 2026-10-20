# Keyboard reactions

The daemon can overlay a short per-key glow on any existing lighting effect.
The selected color fades back to the base effect over 50..5000 ms. Global and
per-device brightness/correction are applied afterwards, so brightness zero stays
dark. Repeated keydowns refresh that key's pulse. There is no held-key lock or
permanent latch; pulses expire even after a lost event or listener failure.

In Classic, expand **Keyboard reactions** on the Lighting page, select target
keyboards, color, fade and ISO/ANSI geometry, then apply settings. Capture is
disabled by default. **Preview A key** produces a single pulse without listening
to the keyboard (also useful while capture is disabled). This does not change the
selected base effect, layers, profiles or existing `EffectSpec` representation.

Initial built-in mapping: Corsair K70 CORE. Other keyboard descriptions may add
explicit `key_usage` per LED; see `PLUGIN_SYSTEM.md`. The adapter exposes a cached
USB keyboard usage -> layout LED index map. Never infer a keyboard key from an RGB
wire index: Corsair modifiers have different indices. Fn has no ordinary OS event
and is intentionally unmapped; the unverified ABNT-specific key is omitted. The
initial event translator covers ordinary set-1/ISO keyboard keys, modifiers,
F1..F12 and keypad/navigation. Media/consumer-page and some international
keys need additional verified mapping. ISO mode directs the Enter-adjacent key to
the ISO LED; ANSI mode uses its ANSI equivalent. Physical scan positions are used
rather than typed characters, preserving positions across QWERTZ/AZERTY layouts.

## Capture and scope

Windows capture runs as an ordinary per-user child of the daemon:
`freergb-keyboard-input.exe --listen`. It uses the Windows low-level keyboard hook
with a dedicated message loop and a bounded nonblocking callback queue; it always
forwards input and never suppresses or generates keys. The callback ignores
OS-injected macro events. It sends only numeric scan positions, extended-key
flags and age over private stdio, not typed text, window titles or application
content. There is no event RPC endpoint, key log, disk history or network output.
Only configuration is saved. No PawnIO, mainboard service or elevation is used.
The helper uses the OS's .NET Framework 4.x APIs, not LHM's optional 4.8 runtime.

Hook input combines **all local keyboards**, mirrored only onto explicitly
selected target RGB keyboards. Physical input-device identity is not determined
by this backend; it does not claim that a particular USB keyboard originated an
event. Capture needs enabled configuration plus a connected target with an explicit
key map. It stops when disabled, targets disappear, conflicting RGB software
suspends target use or the daemon exits. Closing a GUI leaves the daemon and its
explicitly enabled reactions running. `--no-hardware` blocks native capture.
Linux reports `unsupported_platform`; per-key preview/rendering remains portable.

Ready/heartbeat/press records use internal version 1, at most 512 bytes per line,
3-second heartbeat deadline, 256 queued events in both processes and a 250-ms
maximum event age. Each press includes its UTC capture time in milliseconds;
the receiver includes time buffered in the pipe when checking age. Future
timestamps after clock adjustments are discarded. Timestamps are transient and
are never saved. Stale queued events are discarded, not replayed. Native I/O,
startup and EOF/kill cleanup happen outside the render/API lock; stdin EOF releases
the hook, with a bounded kill fallback. A failed capture requires reapplying
settings to retry; it does not enter a silent restart loop. Windows can remove a
hook after an OS timeout without notifying its owner; heartbeat verifies the
process, not independently that Windows retained the hook. Reapply settings after
such a failure. See Microsoft's
[low-level hook lifecycle guidance](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc).

## Configuration and API

Daemon-owned `keyboard-reactive.json` lives beside `state.json` in the usual
configuration root (`%APPDATA%/freeRGB` or `$XDG_CONFIG_HOME/freergb`). It is JSON
version 1, bounded to 128 KiB:

```json
{"version":1,"enabled":false,"color":{"r":255,"g":255,"b":255},"fade_ms":650,"iso":true,"devices":[]}
```

`devices` is an array/set of at most 64 device IDs; each nonempty ID has at most
1024 UTF-8 bytes and no control characters. Version defaults to 1 if omitted;
other fields are required. Unknown fields are ignored for additive client
compatibility. Invalid/oversized files are ignored with an explicit status error;
save validates before replacing existing state. `--no-state` keeps configuration
in memory. Both GUIs configure it solely through the daemon API; no GUI helper,
hook, raw input logic or file writer is introduced. `desktop-ui.json` is untouched.

Additive methods are documented in `LOCAL_API.md`: `keyboard.reactive.get`,
`keyboard.reactive.configure`, `keyboard.reactive.preview`, `keyboard.reactive.disable`. Classic sliders edit
locally; only Apply sends configure. It is an ordered save action, not a stream
of slider setters; `is_setter()` is unchanged. Existing `DeviceSummary`, status,
diagnostics, sensor readings, effect variants and API version remain unchanged.

**Stop reactions now** clears runtime capture/pulses even if saving the disabled
state fails. The returned `persisted` flag/status error distinguishes that case;
an old enabled file may still take effect after a restart until it can be saved.

## Verification limits

Pure tests cover physical keys versus language-dependent virtual keys, keypad/
modifier distinctions, fading/expiration, base-effect/brightness preservation,
protocol handshake, bounds and stale-event rejection. No input history is retained.
`scripts/test-keyboard-helper.ps1` compiles and tests serialization without capture;
`-CaptureLifecycle` is restricted to a disposable GitHub Windows runner, installs
the hook and immediately releases it on EOF without typing or saving events.
Real typing-to-LED behavior across layouts/devices still needs interactive hardware
acceptance; no global hook was enabled on the developer PC by these tests.
