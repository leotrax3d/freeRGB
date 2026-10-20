import { useEffect, useState } from "react"
import {
  Activity, AudioLines, Blend, Cable, Download, Github, Keyboard,
  Layers3, MonitorUp, Palette, PlugZap, Save, ShieldCheck, SlidersHorizontal,
  Monitor, Terminal,
} from "lucide-react"
import appIcon from "../../assets/icons/freergb.png"
import lightLogo from "../../assets/light.svg"
import flatLogo from "../../assets/light_no-gradient.svg"
import monochromeLogo from "../../assets/monochrome.svg"
import refinedLogo from "../../assets/refined.svg"
import { Button } from "./components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "./components/ui/card"
import { Separator } from "./components/ui/separator"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs"

const repo = "https://github.com/leotrax3d/FreeRGB"
const logoChoices = [appIcon, lightLogo, flatLogo, monochromeLogo, refinedLogo]

function pickLogo() {
  if (typeof window === "undefined") return logoChoices[0]
  const previous = window.sessionStorage.getItem("freergb-logo")
  const available = logoChoices.filter((logo) => logo !== previous)
  const selected = available[Math.floor(Math.random() * available.length)]
  window.sessionStorage.setItem("freergb-logo", selected)
  return selected
}

const activeLogo = pickLogo()

const features = [
  { icon: Palette, title: "RGB-Effekte", text: "11 integrierte Effekte: Solid, Rainbow Wave, Breathing, Gradient, Colour Cycle, Scanner, Twinkle und mehr." },
  { icon: Layers3, title: "Effekt-Editor", text: "Eigene Effekte aus bis zu acht Ebenen, Regionen, Deckkraft und verschiedenen Blend-Modi erstellen." },
  { icon: AudioLines, title: "Audio-Reaktivität", text: "Audio Pulse und Audio Spectrum reagieren unter Windows direkt auf die lokale Systemwiedergabe." },
  { icon: MonitorUp, title: "Screen Ambience", text: "Farben des Hauptmonitors werden mit etwa 30 Bildern pro Sekunde auf das RGB-Layout übertragen." },
  { icon: Blend, title: "Geräte-Layout", text: "Geräte per Drag-and-drop auf einem gemeinsamen Effekt-Canvas positionieren und gemeinsam beleuchten." },
  { icon: Save, title: "Profile", text: "Beleuchtung, Helligkeit und Effekte als benannte Profile speichern und wiederherstellen." },
  { icon: Keyboard, title: "Makros", text: "Abläufe für Farbe, Helligkeit, Effekte, Verzögerungen, Texte, Tasten und Tastenkombinationen bauen." },
  { icon: Activity, title: "Monitoring", text: "Verfügbare Drehzahlen und Temperaturen mit Verlauf, lokalen Warnregeln und Hysterese anzeigen." },
  { icon: SlidersHorizontal, title: "Lüfterkurven", text: "Kurven mit Interpolation, Mindestleistung, kritischer Temperatur und Fallback-Verhalten simulieren." },
  { icon: Cable, title: "Lokale API", text: "JSON-RPC per Windows Named Pipe oder Unix Socket – ohne offenen Netzwerk-Port." },
  { icon: PlugZap, title: "Plugin-System", text: "Deklarative TOML-Gerätebeschreibungen und aktivierbare Prozess-Plugins für Erweiterungen." },
  { icon: ShieldCheck, title: "Hardware-Sicherheit", text: "Nur bekannte Adapter werden beschrieben; Rückgabe und Konflikterkennung sind eingebaut." },
]

export default function App() {
  const [platform, setPlatform] = useState("windows")

  useEffect(() => {
    const value = `${navigator.userAgent} ${navigator.platform}`.toLowerCase()
    if (value.includes("linux") && !value.includes("android")) setPlatform("linux")

    let favicon = document.querySelector<HTMLLinkElement>("link[rel='icon']")
    if (!favicon) {
      favicon = document.createElement("link")
      favicon.rel = "icon"
      document.head.appendChild(favicon)
    }
    favicon.href = activeLogo
  }, [])

  return <div className="min-h-screen bg-background text-foreground">
    <header className="border-b">
      <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-5 lg:px-8">
        <a href="#" className="flex items-center gap-2 text-sm font-semibold">
          <img src={activeLogo} alt="freeRGB" className="size-8 rounded-md object-contain" />
          freeRGB
        </a>
        <nav className="hidden items-center gap-6 text-sm text-muted-foreground sm:flex">
          <a className="text-foreground" href="#funktionen">Funktionen</a>
          <a className="transition-colors hover:text-foreground" href="#download">Download</a>
          <a className="transition-colors hover:text-foreground" href={`${repo}/blob/main/docs/INSTALL.md`}>Installation</a>
          <a className="transition-colors hover:text-foreground" href={`${repo}/blob/main/docs/DEVICES.md`}>Geräte</a>
        </nav>
        <Button size="sm" variant="outline" asChild>
          <a href={repo} target="_blank" rel="noreferrer"><Github data-icon="inline-start" />GitHub</a>
        </Button>
      </div>
    </header>

    <main id="funktionen" className="mx-auto max-w-7xl px-5 py-20 lg:px-8 lg:py-28">
      <section className="mx-auto flex max-w-3xl flex-col items-center gap-6 text-center">
        <img src={activeLogo} alt="freeRGB Logo" className="size-20 rounded-2xl object-contain" />
        <h1 className="text-balance text-4xl font-bold tracking-tight sm:text-6xl">Alle Funktionen von freeRGB</h1>
        <p className="max-w-2xl text-pretty text-lg leading-8 text-muted-foreground">Offene, lokale RGB-Steuerung ohne Cloud-Konto und Telemetrie. Der aktuelle Funktionsumfang auf einen Blick.</p>
      </section>

      <section id="download" className="mx-auto mt-16 max-w-4xl">
        <Tabs value={platform} onValueChange={setPlatform}>
          <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
            <div className="flex flex-col gap-1"><h2 className="text-2xl font-semibold tracking-tight">freeRGB installieren</h2><p className="text-sm text-muted-foreground">Der passende Installationsweg wurde für dein System vorausgewählt.</p></div>
            <TabsList>
              <TabsTrigger value="windows"><Monitor />Windows</TabsTrigger>
              <TabsTrigger value="linux"><Terminal />Linux</TabsTrigger>
            </TabsList>
          </div>

          <TabsContent value="windows" className="mt-6">
            <Card>
              <CardHeader><CardTitle>Windows 10 / 11</CardTitle><CardDescription>Der Installer wird derzeit über den GitHub-Paket-Workflow erzeugt. Ein öffentlicher Release ist noch nicht verfügbar.</CardDescription></CardHeader>
              <CardContent className="flex flex-col gap-4">
                <div className="rounded-xl bg-muted p-4 font-mono text-sm text-muted-foreground">git clone https://github.com/leotrax3d/FreeRGB.git<br />cd FreeRGB<br />cargo install --path crates/freergb-gui</div>
                <p className="text-sm leading-6 text-muted-foreground">Für den normalen HID-Zugriff werden keine Administratorrechte benötigt. SignalRGB und iCUE müssen vor der Hardware-Nutzung geschlossen werden.</p>
              </CardContent>
              <CardFooter className="flex flex-col gap-3 sm:flex-row">
                <Button asChild><a href={`${repo}/actions/workflows/package.yml`} target="_blank" rel="noreferrer"><Download data-icon="inline-start" />Windows-Builds öffnen</a></Button>
                <Button variant="outline" asChild><a href={`${repo}/archive/refs/heads/main.zip`}><Download data-icon="inline-start" />Quellcode als ZIP</a></Button>
              </CardFooter>
            </Card>
          </TabsContent>

          <TabsContent value="linux" className="mt-6">
            <Card>
              <CardHeader><CardTitle>Linux</CardTitle><CardDescription>Linux wird gebaut und getestet, die Nutzung mit echter Hardware ist derzeit noch nicht validiert.</CardDescription></CardHeader>
              <CardContent className="flex flex-col gap-4">
                <div className="rounded-xl bg-muted p-4 font-mono text-sm text-muted-foreground">git clone https://github.com/leotrax3d/FreeRGB.git<br />cd FreeRGB<br />cargo install --path crates/freergb-gui</div>
                <p className="text-sm leading-6 text-muted-foreground">Zusätzlich werden die in der Installationsdokumentation aufgeführten Systembibliotheken und udev-Regeln benötigt.</p>
              </CardContent>
              <CardFooter className="flex flex-col gap-3 sm:flex-row">
                <Button asChild><a href={`${repo}/archive/refs/heads/main.tar.gz`}><Download data-icon="inline-start" />Quellcode als TAR.GZ</a></Button>
                <Button variant="outline" asChild><a href={`${repo}/blob/main/docs/INSTALL.md#linux`} target="_blank" rel="noreferrer"><Terminal data-icon="inline-start" />Linux-Anleitung</a></Button>
              </CardFooter>
            </Card>
          </TabsContent>
        </Tabs>
      </section>

      <Separator className="my-16" />

      <section className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {features.map(({ icon: Icon, title, text }) =>
          <Card key={title}>
            <CardHeader>
              <div className="flex size-10 items-center justify-center rounded-lg bg-muted"><Icon className="size-5" /></div>
              <CardTitle className="pt-4">{title}</CardTitle>
              <CardDescription>{text}</CardDescription>
            </CardHeader>
          </Card>
        )}
      </section>

      <section className="mt-16 flex flex-col gap-5 rounded-xl border bg-card p-6 sm:flex-row sm:items-center sm:justify-between sm:p-8">
        <div className="flex flex-col gap-1"><h2 className="font-semibold">Frühe Entwicklungsphase</h2><p className="max-w-2xl text-sm leading-6 text-muted-foreground">Corsair K70 CORE ist validiert. Der iCUE LINK Hub ist experimentell und muss ausdrücklich aktiviert werden.</p></div>
        <Button asChild><a href={`${repo}/blob/main/docs/DEVICES.md`} target="_blank" rel="noreferrer">Hardware-Status</a></Button>
      </section>
    </main>

    <footer className="border-t"><div className="mx-auto flex max-w-7xl flex-col gap-2 px-5 py-8 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between lg:px-8"><span>freeRGB</span><span>MIT oder Apache 2.0 · Unabhängiges Open-Source-Projekt.</span></div></footer>
  </div>
}
