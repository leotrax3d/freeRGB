import { useEffect, useState, type ReactNode } from "react"
import {
  Activity, ArrowUpRight, AudioLines, Cable, Check, CircleHelp, Cpu, Download, Fan,
  Github, Keyboard, Layers, LayoutDashboard, Monitor, MonitorUp, Package, PlugZap,
  Search, Settings, ShieldCheck, Sparkles, Terminal, type LucideIcon,
} from "lucide-react"
import wordmark from "../../assets/brand/wordmark-dark.svg"
import appIcon from "../../assets/icons/freergb-32.png"
import bundledWebsiteContent from "../../docs/WEBSITE_CONTENT.md?raw"
import { LiveDemo } from "./components/LiveDemo"

const repo = "https://github.com/leotrax3d/FreeRGB"

type Platform = "windows" | "linux" | "other"
type Feature = {
  icon: LucideIcon
  title: string
  color: "red" | "green" | "blue" | "purple" | "cyan" | "amber"
}

type WebsiteData = {
  featureDetails: Record<string, { description: string; details: string[] }>
  devices: { name: string; id: string; scope: string }[]
  downloads: Record<"windows" | "linux" | "source", { label: string; url: string; note: string }>
  faq: { question: string; answer: string }[]
}

type Group = {
  id: string
  nav: string
  title: string
  description: string
  icon: LucideIcon
  features: Feature[]
}

const groups: Group[] = [
  {
    id: "light", nav: "Light", title: "Light", icon: Sparkles,
    description: "Create one lighting system from every supported device.",
    features: [
      { icon: Sparkles, title: "Lighting", color: "green" },
      { icon: LayoutDashboard, title: "Layout", color: "purple" },
      { icon: Layers, title: "Profiles", color: "blue" },
    ],
  },
  {
    id: "hardware", nav: "Hardware", title: "Hardware", icon: Cpu,
    description: "See what is connected, what it is doing, and how it is being cooled.",
    features: [
      { icon: Cpu, title: "Devices", color: "blue" },
      { icon: Fan, title: "Cooling", color: "cyan" },
      { icon: Activity, title: "Monitoring", color: "amber" },
    ],
  },
  {
    id: "automation", nav: "Automation", title: "Automation", icon: Keyboard,
    description: "Make lighting respond to sound, the screen, or a sequence you define.",
    features: [
      { icon: AudioLines, title: "Audio reactive", color: "red" },
      { icon: MonitorUp, title: "Screen ambience", color: "purple" },
      { icon: Keyboard, title: "Macros", color: "green" },
    ],
  },
  {
    id: "platform", nav: "Open platform", title: "Open platform", icon: PlugZap,
    description: "Extend freeRGB without giving up local control or opening a network service.",
    features: [
      { icon: Cable, title: "Local API", color: "blue" },
      { icon: PlugZap, title: "Plugin system", color: "purple" },
      { icon: ShieldCheck, title: "Safety and privacy", color: "green" },
    ],
  },
]

type ReleaseInfo = {
  tag: string
  name: string
  url: string
  publishedAt: string
  notes: string
  assets: { name: string; url: string }[]
}

function parseWebsiteContent(markdown: string): WebsiteData {
  const json = markdown.match(/```json\s*([\s\S]*?)```/i)?.[1]
  if (!json) throw new Error("WEBSITE_CONTENT.md does not contain a JSON block")
  return JSON.parse(json) as WebsiteData
}

const bundledWebsiteData = parseWebsiteContent(bundledWebsiteContent)

function SideLink({ href, icon: Icon, active, children }: { href: string; icon: LucideIcon; active: boolean; children: ReactNode }) {
  return <a className={`side-link${active ? " active" : ""}`} href={href} aria-current={active ? "location" : undefined}><Icon size={16} /><span>{children}</span></a>
}

function FeatureRow({ feature, content }: { feature: Feature; content: WebsiteData["featureDetails"][string] }) {
  const Icon = feature.icon
  return <article className="feature-row">
    <div className={`feature-mark ${feature.color}`}><Icon size={19} /></div>
    <div className="feature-copy"><h3>{feature.title}</h3><p>{content.description}</p></div>
    <ul>{content.details.map((detail) => <li key={detail}><Check size={12} />{detail}</li>)}</ul>
  </article>
}

export default function App() {
  const [activeSection, setActiveSection] = useState("overview")
  const [platform, setPlatform] = useState<Platform>("other")
  const [release, setRelease] = useState<ReleaseInfo | null | undefined>(undefined)
  const [websiteData, setWebsiteData] = useState<WebsiteData>(bundledWebsiteData)
  const sections = [
    { id: "overview", label: "Overview", icon: Sparkles },
    { id: "demo", label: "Live demo", icon: Monitor },
    ...groups.slice(0, 3).map(({ id, nav: label, icon }) => ({ id, label, icon })),
    { id: "compatibility", label: "Compatibility", icon: Search },
    { id: "platform", label: "Open platform", icon: PlugZap },
    { id: "install", label: "Install", icon: Download },
    { id: "faq", label: "FAQ", icon: CircleHelp },
  ]
  const activeLabel = sections.find(({ id }) => id === activeSection)?.label ?? "Overview"
  const windowsAsset = release?.assets.find(({ name }) => /windows.*(setup|installer).*\.exe$/i.test(name) || /setup\.exe$/i.test(name))
  const installTarget = platform === "windows"
    ? { label: windowsAsset ? "Download for Windows" : "View Windows builds", href: windowsAsset?.url ?? websiteData.downloads.windows.url }
    : platform === "linux"
      ? { label: "Install on Linux", href: websiteData.downloads.linux.url }
      : { label: "View install options", href: "#install" }

  useEffect(() => {
    let favicon = document.querySelector<HTMLLinkElement>("link[rel='icon']")
    if (!favicon) { favicon = document.createElement("link"); favicon.rel = "icon"; document.head.appendChild(favicon) }
    favicon.href = appIcon

    const system = `${navigator.userAgent} ${navigator.platform}`.toLowerCase()
    if (system.includes("windows")) setPlatform("windows")
    else if (system.includes("linux") && !system.includes("android")) setPlatform("linux")
  }, [])

  useEffect(() => {
    fetch("https://raw.githubusercontent.com/leotrax3d/FreeRGB/main/docs/WEBSITE_CONTENT.md", { cache: "no-cache" })
      .then((response) => {
        if (!response.ok) throw new Error(`GitHub returned ${response.status}`)
        return response.text()
      })
      .then((markdown) => setWebsiteData(parseWebsiteContent(markdown)))
      .catch(() => { /* The bundled Markdown remains the offline/private-repository fallback. */ })
  }, [])

  useEffect(() => {
    fetch("https://api.github.com/repos/leotrax3d/FreeRGB/releases/latest", { headers: { Accept: "application/vnd.github+json" } })
      .then(async (response) => {
        if (response.status === 404) return null
        if (!response.ok) throw new Error(`GitHub returned ${response.status}`)
        const data = await response.json() as { tag_name: string; name: string | null; html_url: string; published_at: string; body: string | null; assets: { name: string; browser_download_url: string }[] }
        return { tag: data.tag_name, name: data.name || data.tag_name, url: data.html_url, publishedAt: data.published_at, notes: data.body || "No release notes were provided.", assets: data.assets.map((asset) => ({ name: asset.name, url: asset.browser_download_url })) }
      })
      .then(setRelease)
      .catch(() => setRelease(null))
  }, [])

  useEffect(() => {
    let queued = false
    const update = () => {
      if (queued) return
      queued = true
      requestAnimationFrame(() => {
        const marker = window.scrollY + window.innerHeight * 0.32
        let current = sections[0].id
        for (const { id } of sections) {
          const element = document.getElementById(id)
          if (element && element.offsetTop <= marker) current = id
        }
        setActiveSection(current)
        queued = false
      })
    }
    update()
    window.addEventListener("scroll", update, { passive: true })
    window.addEventListener("resize", update)
    return () => { window.removeEventListener("scroll", update); window.removeEventListener("resize", update) }
  }, [])

  return <div className="site-frame">
    <header className="mobile-header">
      <div className="mobile-top"><a href="#overview"><img src={wordmark} alt="freeRGB" /></a><span>{activeLabel}</span><a className="icon-link" href={repo} target="_blank" rel="noreferrer" aria-label="Open GitHub"><Github size={18} /></a></div>
      <nav aria-label="Current website section">{sections.map(({ id, label }) => <a key={id} href={`#${id}`} className={activeSection === id ? "active" : ""} aria-current={activeSection === id ? "location" : undefined}>{label}</a>)}</nav>
    </header>

    <aside className="sidebar">
      <a className="wordmark" href="#overview"><img src={wordmark} alt="freeRGB" /></a>
      <nav aria-label="Website sections">
        <span className="nav-label">Product</span>
        <SideLink href="#overview" icon={Sparkles} active={activeSection === "overview"}>Overview</SideLink>
        <SideLink href="#demo" icon={Monitor} active={activeSection === "demo"}>Live demo</SideLink>
        {groups.slice(0, 3).map((group) => <SideLink key={group.id} href={`#${group.id}`} icon={group.icon} active={activeSection === group.id}>{group.nav}</SideLink>)}
        <span className="nav-label second">Project</span>
        <SideLink href="#compatibility" icon={Search} active={activeSection === "compatibility"}>Compatibility</SideLink>
        <SideLink href="#platform" icon={PlugZap} active={activeSection === "platform"}>Open platform</SideLink>
        <SideLink href="#install" icon={Download} active={activeSection === "install"}>Install</SideLink>
        <SideLink href="#faq" icon={CircleHelp} active={activeSection === "faq"}>FAQ</SideLink>
        <a className="side-link" href={`${repo}/blob/main/docs/INSTALL.md`} target="_blank" rel="noreferrer"><Settings size={16} /><span>Documentation</span><ArrowUpRight className="external" size={13} /></a>
      </nav>
      <div className="sidebar-spacer" />
      <a className="github-link" href={repo} target="_blank" rel="noreferrer"><Github size={16} />View on GitHub</a>
    </aside>

    <main className="workspace">
      <section id="overview" className="overview view-section">
        <div className="overview-copy">
          <h1>Control lighting, cooling, and devices from one place.</h1>
          <p className="lead">freeRGB runs on your machine and talks directly to supported hardware. It does not require an account, telemetry, or an open network port.</p>
          <div className="actions"><a className="button primary" href="#light">Explore features</a><a className="button secondary" href={installTarget.href} target={platform === "other" ? undefined : "_blank"} rel={platform === "other" ? undefined : "noreferrer"}><Download size={15} />{installTarget.label}</a></div>
        </div>
        <div className="overview-index">
          {groups.map((group) => <a href={`#${group.id}`} key={group.id}><group.icon size={17} /><span><b>{group.title}</b><small>{group.description}</small></span><ArrowUpRight size={14} /></a>)}
        </div>
      </section>

      <LiveDemo />
      {groups.slice(0, 3).map((group) => <section id={group.id} className="catalog view-section" key={group.id}>
        <header className="view-header"><div className="view-symbol"><group.icon size={18} /></div><div><h2>{group.title}</h2><p>{group.description}</p></div></header>
        <div className="feature-list">{group.features.map((feature) => <FeatureRow key={feature.title} feature={feature} content={websiteData.featureDetails[feature.title]} />)}</div>
      </section>)}

      <section id="compatibility" className="catalog view-section">
        <header className="view-header"><div className="view-symbol"><Search size={18} /></div><div><h2>Hardware support</h2><p>See which hardware classes and capabilities are currently available.</p></div></header>
        <div className="device-list">
          <div className="device-head"><span>Hardware class</span><span>Current scope</span></div>
          {websiteData.devices.map((device) => <article className="device-row" key={device.name}><div><b>{device.name}</b><code>{device.id}</code></div><p>{device.scope}</p></article>)}
        </div>
        <p className="section-note">Availability depends on the underlying protocol and safety validation. See the device documentation for exact limitations.</p>
      </section>

      {groups.slice(3).map((group) => <section id={group.id} className="catalog view-section" key={group.id}>
        <header className="view-header"><div className="view-symbol"><group.icon size={18} /></div><div><h2>{group.title}</h2><p>{group.description}</p></div></header>
        <div className="feature-list">{group.features.map((feature) => <FeatureRow key={feature.title} feature={feature} content={websiteData.featureDetails[feature.title]} />)}</div>
      </section>)}

      <section id="install" className="install view-section">
        <header className="view-header"><div className="view-symbol"><Download size={18} /></div><div><h2>Install freeRGB</h2><p>The recommended download is selected for this operating system.</p></div></header>
        <div className="release-panel">
          <div className="release-icon"><Package size={20} /></div>
          {release === undefined ? <div><h3>Checking GitHub Releases…</h3><p>Version and release notes are loaded directly from GitHub.</p></div>
            : release === null ? <div><h3>No published release yet</h3><p>Installers are currently produced by GitHub Actions. The website will switch to the published installer automatically when the first release is available.</p></div>
              : <div><p><time dateTime={release.publishedAt}>{new Date(release.publishedAt).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" })}</time></p><h3>{release.name} <small>{release.tag}</small></h3><p>{release.notes}</p><a href={release.url} target="_blank" rel="noreferrer">Full release notes <ArrowUpRight size={13} /></a></div>}
        </div>
        <div className="install-list">
          <a className={platform === "windows" ? "recommended" : ""} href={windowsAsset?.url ?? websiteData.downloads.windows.url} target="_blank" rel="noreferrer"><Monitor size={20} /><span><b>{websiteData.downloads.windows.label}</b><small>{platform === "windows" ? `Recommended · ${windowsAsset ? "Download latest installer" : websiteData.downloads.windows.note}` : websiteData.downloads.windows.note}</small></span><ArrowUpRight size={15} /></a>
          <a className={platform === "linux" ? "recommended" : ""} href={websiteData.downloads.linux.url} target="_blank" rel="noreferrer"><Terminal size={20} /><span><b>{websiteData.downloads.linux.label}</b><small>{platform === "linux" ? `Recommended · ${websiteData.downloads.linux.note}` : websiteData.downloads.linux.note}</small></span><ArrowUpRight size={15} /></a>
          <a href={websiteData.downloads.source.url} target="_blank" rel="noreferrer"><Github size={20} /><span><b>{websiteData.downloads.source.label}</b><small>{websiteData.downloads.source.note}</small></span><ArrowUpRight size={15} /></a>
        </div>
      </section>

      <section id="faq" className="faq view-section">
        <header className="view-header"><div className="view-symbol"><CircleHelp size={18} /></div><div><h2>Frequently asked questions</h2><p>Practical answers about setup, privacy, conflicts, and hardware support.</p></div></header>
        <div className="faq-list">{websiteData.faq.map(({ question, answer }) => <details key={question}><summary>{question}<span>+</span></summary><p>{answer}</p></details>)}</div>
      </section>

      <footer><span>freeRGB</span><span>MIT or Apache 2.0</span><a href={repo}>Source on GitHub</a></footer>
    </main>
  </div>
}
