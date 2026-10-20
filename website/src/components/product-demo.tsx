import { useState, type ReactNode } from "react"
import { Activity, Keyboard, Layers3, Palette, Play, Save } from "lucide-react"
import { Badge } from "./ui/badge"
import { Button } from "./ui/button"
import { cn } from "../lib/utils"
import { Slider } from "./ui/slider"
import { Switch } from "./ui/switch"
import { Tabs, TabsList, TabsTrigger } from "./ui/tabs"
import { Separator } from "./ui/separator"

const effects = [
  { name: "Rainbow Wave", hue: 292, speed: "Mittel" },
  { name: "Breathing", hue: 190, speed: "Langsam" },
  { name: "Audio Spectrum", hue: 145, speed: "Live" },
  { name: "Twinkle", hue: 255, speed: "Ruhig" }
]
const tabs = [
  ["lighting", "Beleuchtung", Palette], ["layout", "Layout", Layers3],
  ["monitoring", "Monitoring", Activity], ["macros", "Makros", Keyboard]
] as const

export function ProductDemo() {
  const [tab, setTab] = useState("lighting")
  const [active, setActive] = useState(0)
  const [brightness, setBrightness] = useState(72)
  const [enabled, setEnabled] = useState(true)
  const effect = effects[active]

  return <section id="demo" className="mx-auto max-w-7xl px-5 py-28 lg:px-8">
    <div className="mx-auto max-w-2xl text-center"><Badge>Interaktive Produktdemo</Badge><h2 className="mt-5 font-display text-4xl font-bold tracking-tight sm:text-5xl">Probier das UI direkt aus.</h2><p className="mt-5 text-lg leading-8 text-zinc-400">Wechsle Bereiche, wähle Effekte und regle die Helligkeit. Die Demo bildet den Aufbau der nativen Anwendung vereinfacht im Browser ab.</p></div>
    <div className="mt-14 overflow-hidden rounded-[1.75rem] border border-white/10 bg-[#0b0b0e] shadow-2xl shadow-violet-950/30">
      <div className="flex items-center justify-between border-b border-white/10 px-5 py-3"><div className="flex gap-2"><i className="size-2.5 rounded-full bg-rose-400"/><i className="size-2.5 rounded-full bg-amber-300"/><i className="size-2.5 rounded-full bg-emerald-400"/><span className="ml-3 hidden text-xs text-zinc-500 sm:block">freeRGB · Desktop</span></div><span className="flex items-center gap-2 text-xs text-zinc-500"><i className="size-1.5 rounded-full bg-emerald-400 shadow-[0_0_8px_#34d399]"/>Dienst verbunden</span></div>
      <Tabs value={tab} onValueChange={setTab} className="grid min-h-[600px] md:grid-cols-[210px_1fr]">
        <aside className="border-b border-white/10 bg-white/[.02] p-3 md:border-b-0 md:border-r md:p-4"><div className="mb-4 hidden rounded-xl border border-white/10 bg-black/20 p-3 md:block"><small className="uppercase tracking-widest text-zinc-600">Profil</small><div className="mt-1 text-sm font-semibold">Aurora Night</div></div><TabsList className="grid h-auto grid-cols-4 gap-1 bg-transparent p-0 md:block md:space-y-1">{tabs.map(([id,label,Icon])=><TabsTrigger key={id} value={id} className="flex w-full flex-col items-center gap-1.5 rounded-lg px-2 py-2.5 text-[10px] text-zinc-500 shadow-none data-[state=active]:bg-white data-[state=active]:text-zinc-950 md:flex-row md:justify-start md:gap-3 md:px-3 md:text-sm"><Icon className="size-4"/>{label}</TabsTrigger>)}</TabsList><Separator className="mt-6 hidden md:block"/><div className="mt-5 hidden md:block"><small className="px-3 uppercase tracking-widest text-zinc-600">Geräte</small><p className="mt-4 px-3 text-xs text-zinc-400"><i className="mr-2 inline-block size-1.5 rounded-full bg-emerald-400"/>K70 CORE RGB</p><p className="mt-3 px-3 text-xs text-zinc-400"><i className="mr-2 inline-block size-1.5 rounded-full bg-amber-400"/>iCUE LINK Hub</p></div></aside>
        <div className="p-4 sm:p-6 lg:p-8">
          {tab === "lighting" && <><Header title="Beleuchtung" sub="K70 CORE RGB" action={<Button size="sm" variant="outline"><Save className="size-3.5"/>Profil speichern</Button>}/><div className="mt-6 grid gap-5 lg:grid-cols-[1.2fr_.8fr]"><div className="relative min-h-[330px] overflow-hidden rounded-2xl border border-white/10 bg-black/30 p-6"><div className="absolute inset-0 transition duration-500" style={{opacity:enabled?brightness/100:0,background:`radial-gradient(circle at 25% 35%,hsl(${effect.hue} 85% 55% / .45),transparent 34%),radial-gradient(circle at 75% 58%,hsl(${effect.hue+70} 90% 55% / .38),transparent 42%)`}}/><div className="relative flex justify-between"><Badge className="bg-black/30">Live-Canvas</Badge><Switch checked={enabled} onCheckedChange={setEnabled} aria-label="Beleuchtung umschalten" className="data-[state=checked]:bg-emerald-400"/></div><KeyboardView hue={effect.hue} level={enabled?brightness:0}/></div><div className="space-y-4"><Panel title="Effekt"><div className="grid grid-cols-2 gap-2">{effects.map((e,i)=><button key={e.name} onClick={()=>setActive(i)} className={cn("rounded-lg border p-3 text-left text-xs transition",i===active?"border-white/20 bg-white/10 text-white":"border-white/5 text-zinc-500")}><i className="mb-2 block h-1.5 rounded-full" style={{background:`linear-gradient(90deg,hsl(${e.hue} 85% 55%),hsl(${e.hue+70} 85% 55%))`}}/>{e.name}</button>)}</div></Panel><Panel title="Parameter"><label className="flex justify-between text-xs text-zinc-500">Helligkeit <b className="text-white">{brightness}%</b></label><Slider className="mt-4" min={0} max={100} step={1} value={[brightness]} onValueChange={([value])=>setBrightness(value)}/><p className="mt-5 flex justify-between text-xs text-zinc-500">Geschwindigkeit <b className="text-white">{effect.speed}</b></p></Panel></div></div></>}
          {tab === "layout" && <><Header title="Geräte-Layout" sub="Positioniere Geräte auf dem gemeinsamen Effekt-Canvas."/><div className="mt-6 grid min-h-[450px] place-items-center rounded-2xl border border-white/10 bg-grid p-6"><div className="relative h-72 w-full max-w-2xl"><div className="absolute left-[5%] top-[12%] w-[70%] rounded-xl border border-fuchsia-400/60 bg-fuchsia-400/10 p-4"><span className="text-xs">K70 CORE RGB</span><KeyboardView hue={292} level={75} compact/></div><div className="absolute bottom-[4%] right-[2%] flex gap-2 rounded-xl border border-cyan-400/60 bg-cyan-400/10 p-4"><i className="grid size-12 place-items-center rounded-full border-4 border-cyan-300/50 text-[9px]">FAN 1</i><i className="grid size-12 place-items-center rounded-full border-4 border-violet-300/50 text-[9px]">FAN 2</i></div></div></div></>}
          {tab === "monitoring" && <><Header title="Monitoring" sub="Lokale Telemetrie aus dem Geräte-Cache."/><div className="mt-6 grid gap-4 lg:grid-cols-3">{[["Lüfter 1","1.240","RPM"],["Pumpe","2.180","RPM"],["Sensor","34.2","°C"]].map(([n,v,u],x)=><Panel key={n} title={n}><strong className="text-3xl">{v}</strong><small className="ml-2 text-zinc-500">{u}</small><div className="mt-5 flex h-24 items-end gap-1">{Array.from({length:22}).map((_,i)=><i key={i} className={cn("flex-1 rounded-t-sm",x===0?"bg-cyan-400/60":x===1?"bg-violet-400/60":"bg-emerald-400/60")} style={{height:`${25+((i*13+x*17)%65)}%`}}/>)}</div><small className="mt-3 block text-zinc-600">Illustrative Demo-Werte</small></Panel>)}</div><p className="mt-4 rounded-xl border border-amber-400/15 bg-amber-400/5 p-4 text-xs leading-5 text-amber-100/70">Die Werte demonstrieren nur das UI. Die reale LINK-Kanalzuordnung und Live-Akzeptanz sind noch offen.</p></>}
          {tab === "macros" && <><Header title="Makros" sub="Baue Abläufe aus Licht-, Zeit- und Eingabeschritten." action={<Button size="sm"><Play className="size-3.5"/>Ausführen</Button>}/><div className="mt-6 grid gap-5 lg:grid-cols-[.7fr_1.3fr]"><Panel title="Gespeichert"><button className="w-full rounded-lg bg-white p-3 text-left text-sm font-semibold text-zinc-950">Focus Mode<small className="mt-1 block font-normal text-zinc-500">4 Schritte · manuell</small></button><button className="mt-2 w-full rounded-lg p-3 text-left text-sm text-zinc-500">Game Start<small className="mt-1 block">6 Schritte · Ctrl+Alt+F7</small></button></Panel><Panel title="Focus Mode"><div className="space-y-2">{[["Farbe setzen","#7c3aed"],["Helligkeit","45 %"],["Effekt","Breathing"],["Warten","2.000 ms"]].map(([label,value],i)=><div key={label} className="flex items-center gap-3 rounded-lg border border-white/5 bg-black/20 p-3 text-sm"><span className="text-zinc-600">{i+1}</span><span className="flex-1">{label}</span><small className="text-zinc-500">{value}</small></div>)}</div></Panel></div></>}
        </div>
      </Tabs>
    </div><p className="mt-4 text-center text-xs text-zinc-600">Browser-Demo · keine Verbindung zu deiner Hardware</p>
  </section>
}

function Header({title,sub,action}:{title:string,sub:string,action?:ReactNode}) { return <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-center"><div><h3 className="font-display text-2xl font-bold">{title}</h3><p className="mt-1 text-xs text-zinc-500">{sub}</p></div>{action}</div> }
function Panel({title,children}:{title:string,children:ReactNode}) { return <div className="rounded-xl border border-white/10 bg-white/[.025] p-4"><div className="mb-4 text-[10px] font-semibold uppercase tracking-widest text-zinc-500">{title}</div>{children}</div> }
function KeyboardView({hue,level,compact=false}:{hue:number,level:number,compact?:boolean}) { return <div className={cn("relative mt-14 grid grid-cols-15 gap-1.5 transition-opacity",compact&&"mt-5 gap-1")} style={{opacity:level/100}}>{Array.from({length:60}).map((_,i)=><i key={i} className="aspect-square rounded-sm border border-white/10" style={{background:`hsl(${hue+(i%15)*5} 82% ${42+(i%4)*5}%)`,boxShadow:`0 0 ${compact?4:8}px hsl(${hue+(i%15)*5} 82% 55% / .42)`}}/>)}</div> }
