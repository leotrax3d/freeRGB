import { useEffect, useState, type ReactNode } from "react"
import {
  Activity, ArrowUpRight, Play, AudioLines, BookOpen, Cable, Check, CircleHelp, Cpu, Download, Fan,
  Github, Keyboard, Layers, LayoutDashboard, Monitor, MonitorUp, Package, PlugZap,
  ScanSearch, Settings, Sparkles, Terminal, type LucideIcon,
} from "lucide-react"
import wordmark from "../../assets/brand/wordmark-dark.svg"
import appIcon from "../../assets/icons/freergb-32.png"
import bundledWebsiteContent from "../../docs/WEBSITE_CONTENT.md?raw"
import { EffectPlayground } from "./components/EffectPlayground"
import { ProductTour } from "./components/ProductTour"
import { HardwareCheck } from "./components/HardwareCheck"

const repo = "https://github.com/leotrax3d/FreeRGB"

type Platform = "windows" | "linux" | "other"
type Feature = {
  icon: LucideIcon
  title: string
  color: "red" | "green" | "blue" | "purple" | "cyan" | "amber"
}

type WebsiteData = {
  featureDetails: Record<string, { description: string; details: string[] }>
  downloads: Record<"windows" | "linux" | "source", { label: string; url: string; note: string }>
  faq: { question: string; answer: string }[]
}

type Group = {
  id: string
  nav: string
  title: string
  icon: LucideIcon
  features: Feature[]
}

const groups: Group[] = [
  {
    id: "light", nav: "Lighting", title: "Lighting", icon: Sparkles,
    features: [
      { icon: Sparkles, title: "Lighting", color: "green" },
      { icon: LayoutDashboard, title: "Layout", color: "purple" },
      { icon: Layers, title: "Profiles", color: "blue" },
    ],
  },
  {
    id: "hardware", nav: "Hardware", title: "Hardware", icon: Cpu,
    features: [
      { icon: Cpu, title: "Devices", color: "blue" },
      { icon: Fan, title: "Cooling", color: "cyan" },
      { icon: Activity, title: "Monitoring", color: "amber" },
    ],
  },
  {
    id: "automation", nav: "Automation", title: "Automation", icon: Keyboard,
    features: [
      { icon: AudioLines, title: "Audio reactive", color: "red" },
      { icon: MonitorUp, title: "Screen ambience", color: "purple" },
      { icon: Keyboard, title: "Macros", color: "green" },
    ],
  },
  {
    id: "platform", nav: "Open platform", title: "Integrations", icon: PlugZap,
    features: [
      { icon: Cable, title: "Local API", color: "blue" },
      { icon: PlugZap, title: "Plugin system", color: "purple" },
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
  const websiteData = bundledWebsiteData
  const sections = [
    { id: "overview", label: "Overview", icon: Sparkles },
    { id: "demo", label: "Live demo", icon: Monitor },
    ...groups.slice(0, 3).map(({ id, nav: label, icon }) => ({ id, label, icon })),
    { id: "check", label: "Hardware search", icon: ScanSearch },
    { id: "guide", label: "User guide", icon: BookOpen },
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
    // /releases/latest skips prereleases, so take the newest published entry (betas included).
    fetch("https://api.github.com/repos/leotrax3d/FreeRGB/releases?per_page=10", { headers: { Accept: "application/vnd.github+json" } })
      .then(async (response) => {
        if (response.status === 404) return null
        if (!response.ok) throw new Error(`GitHub returned ${response.status}`)
        const list = await response.json() as { draft: boolean; tag_name: string; name: string | null; html_url: string; published_at: string; body: string | null; assets: { name: string; browser_download_url: string }[] }[]
        const data = list.find((entry) => !entry.draft && entry.tag_name.startsWith("v"))
        if (!data) return null
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
        for (const element of document.querySelectorAll<HTMLElement>("main section[id]")) {
          if (element.getBoundingClientRect().top + window.scrollY <= marker) current = element.id
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
    <a className="skip-link" href="#overview">Skip to content</a>
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
        <SideLink href="#check" icon={ScanSearch} active={activeSection === "check"}>Hardware search</SideLink>
        <SideLink href="#guide" icon={BookOpen} active={activeSection === "guide"}>User guide</SideLink>
        <SideLink href="#platform" icon={PlugZap} active={activeSection === "platform"}>Open platform</SideLink>
        <SideLink href="#install" icon={Download} active={activeSection === "install"}>Install</SideLink>
        <SideLink href="#faq" icon={CircleHelp} active={activeSection === "faq"}>FAQ</SideLink>
        <a className="side-link" href={`${repo}/blob/main/docs/INSTALL.md`} target="_blank" rel="noreferrer"><Settings size={16} /><span>Documentation</span><ArrowUpRight className="external" size={13} /></a>
      </nav>
      <div className="sidebar-spacer" />
      <a className="github-link" href={repo} target="_blank" rel="noreferrer"><Github size={16} />View on GitHub</a>
    </aside>

    <main className="workspace">
      <section id="overview" className="overview view-section" aria-labelledby="hero-title">
        <div className="overview-copy">
          <h1 id="hero-title">Control your<br />RGB lighting.</h1>
          <p className="lead">Beta software: set RGB effects and fan curves from one app.</p>
          <div className="actions"><a className="button primary" href={`${import.meta.env.BASE_URL}demo/`} target="_blank" rel="noreferrer"><Play size={15} />Try the live demo</a><a className="button secondary" href={installTarget.href} target={platform === "other" ? undefined : "_blank"} rel={platform === "other" ? undefined : "noreferrer"}><Download size={15} />{installTarget.label}</a></div>
        </div>
        <EffectPlayground />
      </section>

      <ProductTour />
      <section id="check" className="hardware-spotlight view-section">
        <header className="view-header"><div className="view-symbol"><ScanSearch size={20} /></div><div><h2>Hardware search</h2></div></header>
        <HardwareCheck />
      </section>

      {groups.slice(0, 3).map((group) => <section id={group.id} className="catalog view-section" key={group.id}>
        <header className="view-header"><div className="view-symbol"><group.icon size={18} /></div><div><h2>{group.title}</h2></div></header>
        <div className="feature-list">{group.features.map((feature) => <FeatureRow key={feature.title} feature={feature} content={websiteData.featureDetails[feature.title]} />)}</div>
      </section>)}

      <section id="guide" className="catalog view-section">
        <header className="view-header"><div className="view-symbol"><BookOpen size={18} /></div><div><h2>User guide</h2></div></header>
        <div className="install-list">
          <a href={`${repo}/blob/main/docs/USER_GUIDE.md`} target="_blank" rel="noreferrer"><BookOpen size={20} /><span><b>Read the user guide</b><small>Setup, fan control safety and connection troubleshooting</small></span><ArrowUpRight size={15} /></a>
          <a href={`${repo}/blob/main/docs/INSTALL.md`} target="_blank" rel="noreferrer"><Download size={20} /><span><b>Installation details</b><small>Windows installer, source builds, Linux</small></span><ArrowUpRight size={15} /></a>
        </div>
      </section>

      {groups.slice(3).map((group) => <section id={group.id} className="catalog view-section" key={group.id}>
        <header className="view-header"><div className="view-symbol"><group.icon size={18} /></div><div><h2>{group.title}</h2></div></header>
        <div className="feature-list">{group.features.map((feature) => <FeatureRow key={feature.title} feature={feature} content={websiteData.featureDetails[feature.title]} />)}</div>
      </section>)}

      <section id="install" className="install view-section">
        <header className="view-header"><div className="view-symbol"><Download size={18} /></div><div><h2>Install freeRGB Beta</h2></div></header>
        {release && <div className="release-panel"><div className="release-icon"><Package size={20} /></div><div><h3>{release.name}</h3><p>{release.notes}</p><a href={release.url} target="_blank" rel="noreferrer">Release notes <ArrowUpRight size={13} /></a></div></div>}
        <div className="install-list">
          <a className={platform === "windows" ? "recommended" : ""} href={windowsAsset?.url ?? websiteData.downloads.windows.url} target="_blank" rel="noreferrer"><Monitor size={20} /><span><b>{websiteData.downloads.windows.label}</b><small>{platform === "windows" ? `Recommended · ${windowsAsset ? "Download latest installer" : websiteData.downloads.windows.note}` : websiteData.downloads.windows.note}</small></span><ArrowUpRight size={15} /></a>
          <a className={platform === "linux" ? "recommended" : ""} href={websiteData.downloads.linux.url} target="_blank" rel="noreferrer"><Terminal size={20} /><span><b>{websiteData.downloads.linux.label}</b><small>{platform === "linux" ? `Recommended · ${websiteData.downloads.linux.note}` : websiteData.downloads.linux.note}</small></span><ArrowUpRight size={15} /></a>
          <a href={websiteData.downloads.source.url} target="_blank" rel="noreferrer"><Github size={20} /><span><b>{websiteData.downloads.source.label}</b><small>{websiteData.downloads.source.note}</small></span><ArrowUpRight size={15} /></a>
        </div>
      </section>

      <section id="faq" className="faq view-section">
        <header className="view-header"><div className="view-symbol"><CircleHelp size={18} /></div><div><h2>Frequently asked questions</h2></div></header>
        <div className="faq-list">{websiteData.faq.map(({ question, answer }) => <details key={question}><summary>{question}<span>+</span></summary><p>{answer}</p></details>)}</div>
      </section>

      <footer><span>freeRGB</span><span>MIT or Apache 2.0</span><a href={repo}>Source on GitHub</a></footer>
    </main>
  </div>
}
