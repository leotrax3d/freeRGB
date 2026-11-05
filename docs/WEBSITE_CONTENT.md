# Website content

This file is the canonical source for website content that changes with the product.
Update it when feature details, hardware support, download routes or FAQ answers
change. The website fetches the current file from GitHub and falls back to
the bundled copy when offline.

Version numbers and release notes do not belong here. They come from GitHub
Releases.

```json
{
  "featureDetails": {
    "Lighting": { "description": "Choose an effect, tune its parameters, and control global brightness from one view.", "details": ["11 built-in effects", "Up to 8 custom layers", "Colour, speed, direction, and scale"] },
    "Layout": { "description": "Place devices on a shared canvas so effects move naturally across the whole desk.", "details": ["Drag-and-drop placement", "Device regions", "Shared effect coordinates"] },
    "Profiles": { "description": "Save complete lighting states and switch between them without rebuilding your setup.", "details": ["Effect and brightness", "Named profiles", "Local storage"] },
    "Devices": { "description": "Inspect connected hardware, firmware, status, and per-device brightness.", "details": ["Automatic discovery", "Connection status", "Conflict detection"] },
    "Cooling": { "description": "Control fans and pumps from the temperature sources that matter to your system.", "details": ["Interpolated fan curves", "Minimum output", "Critical-temperature fallback"] },
    "Monitoring": { "description": "Follow temperatures and speeds with history, warning rules, and diagnostics.", "details": ["Sensor history", "Warnings with hysteresis", "Redacted diagnostic export"] },
    "Audio reactive": { "description": "Turn local system audio into pulse and spectrum effects without sending it anywhere.", "details": ["Audio Pulse", "Audio Spectrum", "Windows capture"] },
    "Screen ambience": { "description": "Map the main display onto the device layout at roughly 30 frames per second.", "details": ["Local screen capture", "Shared device canvas", "Windows capture"] },
    "Macros": { "description": "Build repeatable sequences from lighting, timing, text, and keyboard steps.", "details": ["Colour and effect steps", "Text and key combinations", "Native acceptance pending"] },
    "Local API": { "description": "Use JSON-RPC through Windows named pipes or Unix sockets for your own integrations.", "details": ["No open network port", "Structured state and events", "Local clients only"] },
    "Plugin system": { "description": "Add hardware definitions and optional process integrations without changing the core.", "details": ["Declarative TOML devices", "Optional process plugins", "Hardware acceptance pending"] },
    "Safety and privacy": { "description": "Keep control on the machine with defensive hardware access and no account requirement.", "details": ["Known adapters only", "Conflict protection", "No cloud or telemetry"] }
  },
  "devices": [
    {
      "name": "RGB keyboards",
      "id": "USB / HID",
      "scope": "Per-key lighting, brightness control, and common keyboard layouts"
    },
    {
      "name": "Lighting controllers and hubs",
      "id": "USB",
      "scope": "Multiple lighting channels and connected RGB components"
    },
    {
      "name": "Addressable RGB lighting",
      "id": "ARGB",
      "scope": "Colour effects across individually addressable LEDs"
    },
    {
      "name": "Display modules",
      "id": "USB displays",
      "scope": "Hardware can be detected; display content control is not available"
    },
    {
      "name": "Memory lighting",
      "id": "SMBus",
      "scope": "Detection only while safe cross-platform control is evaluated"
    },
    {
      "name": "Fan and pump control",
      "id": "Native hardware bridge",
      "scope": "Protected commissioning path while hardware safeguards are validated"
    }
  ],
  "downloads": {
    "windows": {
      "label": "Windows 10 / 11",
      "url": "https://github.com/leotrax3d/FreeRGB/actions",
      "note": "Latest installer build from GitHub Actions"
    },
    "linux": {
      "label": "Linux",
      "url": "https://github.com/leotrax3d/FreeRGB/blob/main/docs/INSTALL.md#linux",
      "note": "Build and install from source"
    },
    "source": {
      "label": "Source archive",
      "url": "https://github.com/leotrax3d/FreeRGB/archive/refs/heads/main.zip",
      "note": "Current main branch as ZIP"
    }
  },
  "faq": [
    {
      "question": "Can other RGB software run at the same time?",
      "answer": "Software that controls the same hardware should be closed first. freeRGB detects common access conflicts and leaves affected hardware alone until the conflict is resolved."
    },
    {
      "question": "Does freeRGB require an account?",
      "answer": "No. Profiles, settings, macros, and device state stay on your machine."
    },
    {
      "question": "Does freeRGB work without internet access?",
      "answer": "Yes. Lighting, monitoring, automation, and the local API run without an internet connection. Internet access is only needed for downloads and update checks."
    },
    {
      "question": "Which hardware is supported?",
      "answer": "Support is tracked by hardware class and capability. Some integrations provide full control, while others may currently offer limited or detection-only support."
    },
    {
      "question": "Something does not work. Where do I start?",
      "answer": "Read the troubleshooting section of the user guide. It covers missing devices, the motherboard reader that stays in the starting state, and recovery when the daemon pipe is busy."
    },
    {
      "question": "Is Linux hardware access validated?",
      "answer": "Not yet. The shared core, CI, udev rules, and systemd user unit are implemented, but real Linux hardware acceptance and capture parity remain open."
    }
  ]
}
```
