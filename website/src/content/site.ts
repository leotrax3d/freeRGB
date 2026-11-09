// All website copy in one place. Rules: docs/website/REDESIGN_NOTES.md §5.
// Only state what is on main. Untested hardware is labelled "beta / not hardware-tested".
// Never use: safe, certified, tested for board X, protects your hardware, guaranteed.

export const repo = "https://github.com/leotrax3d/freeRGB"
export const blob = (path: string) => `${repo}/blob/main/${path}`

export const nav = [
  { href: "download/", label: "Download" },
  { href: "devices/", label: "Devices" },
  { href: "docs/", label: "Docs" },
  { href: "privacy/", label: "Privacy" },
] as const

export const hero = {
  badge: "Public beta · Windows and Linux",
  title: "One app for your RGB lighting and fans",
  lead:
    "freeRGB runs lighting effects across devices from different vendors and sets fan curves from your temperature sensors. It is written in Rust, open source, lightweight (a 19 MB installer) and needs no account.",
  primaryWindows: "Download for Windows",
  primaryLinux: "Install on Linux",
  secondary: "Try the web demo",
  note: "Beta software. Most devices have not been tested on real hardware yet.",
  screenshotAlt:
    "The freeRGB lighting view: a list of effects with colour previews on the left and speed and direction controls on the right.",
}

export const trust = [
  { title: "MIT or Apache-2.0", text: "Core source on GitHub. Modules ported from OpenRGB are GPL-2.0-or-later." },
  { title: "Runs locally", text: "No account. Online features stay off until you turn them on." },
  { title: "Windows and Linux", text: "Installer for Windows 10 and 11. Linux builds from source." },
  { title: "No tracking by default", text: "No analytics or crash reporting. Download counting and ratings are opt-in." },
]

export const deviceTeaser = {
  title: "Check your hardware first",
  text: "Search by product name or USB ID. Each result shows whether freeRGB supports it. Every supported device is still beta.",
  placeholder: "e.g. K70, Aura, 1b1c:1bfd",
  cta: "Open the device list",
}

export type Feature = {
  id: string
  eyebrow: string
  title: string
  text: string
  points: string[]
  screenshot: "lighting" | "layout" | "cooling" | "monitoring" | "profiles" | "macros" | "devices" | "marketplace" | "settings"
  alt: string
  maturity?: string
}

export const features: Feature[] = [
  {
    id: "layout",
    eyebrow: "Layout",
    title: "Place your devices on one canvas",
    text: "Drag keyboards, fans and strips to where they sit in your case. A wave then moves across the whole setup.",
    points: ["Drag-and-drop placement", "Profiles store lighting and brightness locally"],
    screenshot: "layout",
    alt: "Layout view showing devices placed on a shared canvas.",
  },
  {
    id: "cooling",
    eyebrow: "Cooling",
    title: "Fan curves from the sensors you choose",
    text: "Pick a temperature source, draw a curve and assign it to a fan or pump. Normal settings keep a minimum fan output and request full speed when a temperature is missing or critical. Unrestricted settings can bypass these limits.",
    points: ["Minimum fan output", "Critical-temperature fallback", "Setup wizard identifies each fan channel"],
    screenshot: "cooling",
    alt: "Cooling view with a fan curve editor plotting fan speed against temperature.",
  },
  {
    id: "monitoring",
    eyebrow: "Monitoring",
    title: "Temperatures and fan speeds over time",
    text: "Watch sensor history, set warning thresholds and export a diagnostic report with personal data removed.",
    points: ["Warning thresholds", "Custom sensors", "Redacted diagnostic export"],
    screenshot: "monitoring",
    alt: "Monitoring view with temperature and fan speed charts.",
  },
  {
    id: "automation",
    eyebrow: "Automation",
    title: "Profiles, audio, screen and macros",
    text: "Switch profiles per app, let lighting follow system audio or extend your screen colours to your devices. Audio and screen capture stay on your PC.",
    points: ["Per-app profiles (opt-in)", "Audio-reactive and screen ambience on Windows", "Macro editor (recording still in progress)"],
    screenshot: "profiles",
    alt: "Profiles view listing saved lighting profiles.",
  },
]

export const platform = {
  title: "Built to be extended",
  text: "Other tools can read state and send commands through a local JSON-RPC API over a Windows named pipe or a Unix socket. No network port is opened.",
  points: [
    { title: "Local API", text: "JSON-RPC over named pipe or Unix socket." },
    { title: "Hardware modules", text: "Device support ships as signed, downloadable modules." },
    { title: "Effect plugins", text: "Effects run as sandboxed WebAssembly." },
  ],
}

export const platforms = [
  {
    id: "windows",
    title: "Windows 10 and 11",
    text: "64-bit installer. It installs for all users, so Windows asks for administrator approval once. The app needs the Microsoft Edge WebView2 runtime, which Windows 11 includes.",
  },
  {
    id: "linux",
    title: "Linux",
    text: "Build from source with Rust and Node. A udev rule gives your user access to supported devices, and a systemd user service runs the daemon. Packages are not published yet, and Linux hardware support has not been tested.",
  },
] as const

export const faq = [
  {
    q: "Do I need to uninstall my vendor RGB software?",
    a: "Close it before you use freeRGB with real hardware. Two programs writing to the same device will fight over it. freeRGB detects common conflicts and leaves an affected device alone. The Windows installer can list installed vendor tools and start their uninstallers, but only if you select them.",
  },
  {
    q: "Why does Windows show a SmartScreen warning?",
    a: "The installer has no Authenticode certificate yet, so Windows does not know the publisher. Check the SHA-256 checksum first (see the download page). If it matches, choose More info, then Run anyway.",
  },
  {
    q: "Will it work with my hardware?",
    a: "Search the device list. Most supported devices were implemented from protocol documentation and checked with simulated tests, not on real hardware. Treat every device as beta until it has been tested.",
  },
  {
    q: "Can wrong fan settings damage my PC?",
    a: "Yes. freeRGB can lower fan and pump speeds, and low speeds can overheat components. Keep the minimum output and the critical-temperature fallback on and watch temperatures after every change. Motherboard fan control has not been confirmed on real hardware for most boards.",
  },
  {
    q: "Does freeRGB send any data?",
    a: "Not by default. You do not need an account. Community ratings and download counting are separate options that start switched off. Update checks and effect or module downloads from GitHub happen only when you start them.",
  },
  {
    q: "What does freeRGB cost?",
    a: "Nothing. It is a private, non-commercial open-source project. There is no paid tier.",
  },
  {
    q: "Does it work on Linux?",
    a: "The core, audio and screen capture, input handling, udev rules and the systemd service are implemented and pass fixture tests. Nobody has run it on real Linux hardware yet, and no packages are published. Build it from source if you want to help test.",
  },
  {
    q: "Where do I start when something breaks?",
    a: "The troubleshooting section of the user guide covers missing devices, connection errors and lighting that stays on after closing.",
  },
]

export const download = {
  title: "Download freeRGB",
  lead: "Beta builds for Windows, and source builds for Linux. Check the file before you run it.",
  betaTitle: "This is beta software",
  betaText:
    "freeRGB can change lighting and lower fan and pump speeds. Wrong settings, untested hardware or software faults can cause overheating, data loss or hardware damage. Most devices and boards have not been tested on real hardware. You use it at your own risk.",
  betaLink: "Cooling safeguards and their limits",
  noRelease:
    "No published release was found when this page was built. You can still build freeRGB from source.",
  windowsTitle: "Windows 10 and 11 (64-bit)",
  linuxTitle: "Linux",
  linuxText:
    "There are no Linux packages yet. Build from source with the steps in the install guide, then install the udev rule so your user can reach supported devices.",
  linuxCta: "Linux install guide",
  sourceTitle: "Source code",
  sourceText: "Clone the repository or download the current main branch as a ZIP file.",
  smartscreenTitle: "About the SmartScreen warning",
  smartscreenText:
    "The installer is not signed with an Authenticode certificate, so Windows shows 'Windows protected your PC'. This warning says the publisher is unknown. It says nothing about whether the file was changed. Check the SHA-256 checksum below. If it matches, select More info, then Run anyway.",
  verifyTitle: "Check the download",
  verifyText:
    "Compare the file's SHA-256 checksum with the value on this page or in the release's SHA256SUMS file. Run one of these commands in the folder where you saved the installer:",
  sigTitle: "What the .sig file is for",
  sigText:
    "Each installer has a detached Ed25519 signature next to it. The freeRGB updater checks this signature against a public key built into the app before it installs an update, and refuses the update if the signature is missing or wrong. You do not need the .sig file for a first install from this page. The checksum is enough.",
  sigKeyLabel: "Release public key (Ed25519)",
  releaseNotes: "Release notes",
  allAssets: "All files on GitHub",
}

export const devicesPage = {
  title: "Supported devices",
  lead:
    "Search the devices freeRGB knows about. The list is built from the device modules and the OpenRGB compatibility matrix in the repository.",
  searchLabel: "Search devices",
  placeholder: "Product name, vendor or USB ID (vid:pid)",
  empty: "No device matches your search. Try fewer words or the USB ID from Device Manager.",
  // Owner rule (docs/legal/SAFETY_LIABILITY_TEXTS.md, D9): until hardware acceptance is done,
  // no text may say a specific device or board is confirmed. Verified and implemented share one label.
  statusLabels: {
    supported: "Beta · not hardware-confirmed",
    blocked: "Blocked",
    missing: "Not supported yet",
  },
  statusHelp: {
    supported: "Implemented in freeRGB. Hardware confirmation is still open for every device.",
    blocked: "Waiting for a missing transport (for example SMBus).",
    missing: "Known from OpenRGB but not ported to freeRGB yet.",
  },
  fansTitle: "Fan controller chips",
  fansLead:
    "Motherboard fan control on Windows works through the board's Super I/O chip. The checks so far are simulated; hardware confirmation is still open for every chip.",
  fanStatus: {
    "hardware-verified": "Beta · not hardware-confirmed",
    simulated: "Beta · not hardware-confirmed",
    unsupported: "Read-only / not supported",
  } as Record<string, string>,
  note: "Supported in software does not mean confirmed on your hardware. Report what works and what does not on GitHub.",
}

export const docsPage = {
  title: "User guide",
  lead: "How to install freeRGB, set up lighting and fans, and fix common problems.",
  source: "This guide is docs/USER_GUIDE.md from the repository.",
  more: "More documentation on GitHub",
}

export const privacyPage = {
  title: "Privacy and legal",
  lead: "What this website does with your data, and where to find the project's legal texts.",
  sections: [
    {
      title: "This website",
      body: [
        "This site sets no cookies and runs no analytics or tracking. Fonts, scripts and images are served from this site. Your browser sends no requests to other services while you read these pages.",
        "Release data, download links and checksums are fetched from GitHub when the site is built, not when you visit. Links to GitHub only contact GitHub when you click them.",
        "If you switch between light and dark mode, the choice is saved in your browser's local storage. It never leaves your device.",
        "The site is hosted on GitHub Pages. GitHub receives your IP address and request data when it serves these pages; see GitHub's privacy statement.",
      ],
    },
    {
      title: "Web demo and admin pages",
      body: [
        "The web demo under /demo/ runs entirely in your browser with simulated devices. Its content security policy blocks all network connections.",
        "The maintainer admin panel under /admin/ is a separate app. When it is opened, it connects to Appwrite (fra.cloud.appwrite.io) and keeps its login session in your browser. The website pages never load it.",
        "The demo saves your last view, language and demo profile in your browser's local storage. Nothing is sent anywhere.",
      ],
    },
    {
      title: "The freeRGB app",
      body: [
        "The app works offline and does not need an account. Community ratings and download counting start switched off, and the app contacts GitHub for updates, effects or modules only when you ask it to.",
        "The full privacy notice for the app is still a draft under legal review. You can read the current draft on GitHub.",
      ],
    },
    {
      title: "Licences",
      body: [
        "The freeRGB core is available under the MIT licence or the Apache License 2.0, at your choice. Hardware modules have their own licences; modules ported from OpenRGB are GPL-2.0-or-later. Third-party components and their licences are listed in THIRD_PARTY_NOTICES.md.",
        "This website uses the Inter typeface (SIL Open Font License 1.1) and Lucide icons (ISC licence).",
      ],
    },
    {
      title: "No affiliation",
      body: [
        "freeRGB is a private, non-commercial project. It is not affiliated with, endorsed by or sponsored by any hardware vendor or other RGB software project. Product names are used only to identify compatible hardware.",
      ],
    },
  ],
  links: [
    { label: "Privacy notice for the app (draft)", path: "docs/PRIVACY.md" },
    { label: "Licence (MIT)", path: "LICENSE-MIT" },
    { label: "Licence (Apache-2.0)", path: "LICENSE-APACHE" },
    { label: "Third-party notices", path: "THIRD_PARTY_NOTICES.md" },
    { label: "Security notes", path: "docs/SECURITY.md" },
  ],
}

export const footer = {
  line: "freeRGB is a private, non-commercial open-source project, provided without warranty.",
  affiliation: "Not affiliated with any hardware vendor or RGB software project.",
}
