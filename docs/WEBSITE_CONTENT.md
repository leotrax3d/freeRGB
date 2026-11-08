> Beta software. The public website must visibly label the current release as Beta.

# Website content

This file is the canonical source for website content that changes with the product.
Update it when feature details, hardware support, download routes or FAQ answers
change. The website bundles this file at build time so copy changes ship with the reviewed
website version. Rebuild the website after editing this file.

The website makes no external requests except one: the GitHub releases API call that
fills the install panel (`api.github.com`). Fonts are self-hosted (Inter, OFL, via
`@fontsource-variable/inter`); no CDN, analytics or cookies. The Privacy section in
`website/src/App.tsx` (English only, the site has no i18n) states this and must be
updated together with any change to external requests. Do not add claims that a
board is safe or tested.

Version numbers and release notes do not belong here. They come from GitHub
Releases.

```json
{
  "featureDetails": {
    "Lighting": {
      "description": "Pick an effect and adjust its colour, speed and brightness in one view.",
      "details": [
        "11 built-in effects",
        "Up to 8 custom layers"
      ]
    },
    "Layout": {
      "description": "Place your devices on a shared canvas. Each device shows its part of the effect.",
      "details": [
        "Drag-and-drop placement"
      ]
    },
    "Profiles": {
      "description": "Save your lighting and brightness as a profile, then switch back whenever you like.",
      "details": [
        "Stored locally"
      ]
    },
    "Devices": {
      "description": "Check connected devices, firmware and connection status. Adjust brightness for each device.",
      "details": [
        "Automatic discovery",
        "Conflict detection"
      ]
    },
    "Cooling": {
      "description": "Set fan and pump speeds from your chosen temperature sources.",
      "details": [
        "Minimum fan output",
        "Critical-temperature fallback"
      ]
    },
    "Monitoring": {
      "description": "Track temperatures and fan speeds over time.",
      "details": [
        "Warning thresholds",
        "Diagnostic export with sensitive data removed"
      ]
    },
    "Audio reactive": {
      "description": "Let your lighting follow system audio with pulse and spectrum effects. Audio stays on your machine.",
      "details": [
        "Audio pulse and spectrum",
        "Capture on Windows"
      ]
    },
    "Screen ambience": {
      "description": "Extend the colours on your main display across your device layout at roughly 30 frames per second.",
      "details": [
        "Capture on Windows"
      ]
    },
    "Macros": {
      "description": "Combine lighting changes, delays, text and key presses into a sequence you can run again.",
      "details": [
        "Macro keys and recording",
        "Native hardware validation pending"
      ]
    },
    "Local API": {
      "description": "Connect your own tools through JSON-RPC over Windows named pipes or Unix sockets.",
      "details": [
        "State and event access"
      ]
    },
    "Plugin system": {
      "description": "Add hardware definitions and optional process integrations without changing the core.",
      "details": [
        "TOML hardware definitions",
        "Signed WASM effects with hot reload",
        "Hardware validation pending"
      ]
    },
    "Safety and privacy": {
      "description": "Control hardware locally. freeRGB checks for access conflicts and does not require an account. Online community features are off until you turn them on.",
      "details": [
        "Known adapters only",
        "Conflict protection",
        "No account needed",
        "Community features off by default",
        "No analytics or crash reporting"
      ]
    }
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
      "scope": "Detection only while cross-platform control is evaluated"
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
      "url": "https://github.com/leotrax3d/freeRGB/actions",
      "note": "Installer builds on GitHub Actions; sign-in may be required"
    },
    "linux": {
      "label": "Linux",
      "url": "https://github.com/leotrax3d/freeRGB/blob/main/docs/INSTALL.md#linux",
      "note": "Build and install from source"
    },
    "source": {
      "label": "Source archive",
      "url": "https://github.com/leotrax3d/freeRGB/archive/refs/heads/main.zip",
      "note": "Current main branch as ZIP"
    }
  },
  "faq": [
    {
      "question": "Can other RGB software run at the same time?",
      "answer": "Close other software that controls the same hardware before starting freeRGB. When freeRGB detects a common access conflict, it leaves the affected device alone until you resolve it."
    },
    {
      "question": "Does freeRGB require an account?",
      "answer": "No. Your profiles, settings, macros and device state stay on your machine. The optional online community features (effect ratings and reports) are off by default; if you turn them on, freeRGB creates a random anonymous ID and contacts the community service. Counting your effect downloads is a separate option that is also off by default."
    },
    {
      "question": "Does freeRGB work without internet access?",
      "answer": "Yes. Lighting, monitoring, automation and the local API work offline. Downloading effects or modules from GitHub and checking for updates need internet access. See the privacy notice draft for every network request."
    },
    {
      "question": "Which hardware is supported?",
      "answer": "Search for your exact model in the hardware check. Capabilities vary: some devices have full control, while others support fewer functions or detection only. Protocol tests and confirmation on real hardware are tracked separately."
    },
    {
      "question": "Something does not work. Where do I start?",
      "answer": "Start with the user guide. Its troubleshooting section covers missing devices, a motherboard reader stuck at startup, and a busy connection to the background service."
    },
    {
      "question": "Is Linux hardware access validated?",
      "answer": "Real Linux hardware and package installation have not yet passed acceptance testing. The shared core, audio and screen capture, evdev input capture, udev rules, systemd user service and .deb, .rpm and AppImage packaging are implemented and tested with fixtures."
    }
  ]
}
```
