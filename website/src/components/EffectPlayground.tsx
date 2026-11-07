import { useState } from "react"
import { Check, Pause, Play, Sun } from "lucide-react"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs"
import { Slider } from "./ui/slider"

// Matches the desktop app's effectPreview colours and preview animations.
// This is a visual preview, not a hardware renderer or connection.
const effects = [
  { id: "rainbow", name: "Rainbow wave", background: "linear-gradient(90deg, #ef4444, #f59e0b, #10a37f, #3b82f6, #a855f7, #ef4444)" },
  { id: "breathing", name: "Breathing", background: "radial-gradient(circle at 50% 50%, rgb(0 150 210), rgb(0 0 0) 85%)" },
  { id: "solid", name: "Solid colour", background: "#0096d2" },
] as const

export function EffectPlayground() {
  const [selected, setSelected] = useState<string>("rainbow")
  const [brightness, setBrightness] = useState(80)
  const [playing, setPlaying] = useState(false)
  return <div className="effect-playground">
    <div className="playground-heading"><span>Effect preview</span></div>
    <Tabs value={selected} onValueChange={(value) => { setSelected(value); setPlaying(true) }}>
      {effects.map((effect) => <TabsContent key={effect.id} value={effect.id} className="effect-stage">
        <div className={`effect-colour effect-${effect.id}`} style={{ backgroundImage: effect.id === "solid" ? undefined : effect.background, backgroundColor: effect.id === "solid" ? effect.background : undefined, opacity: brightness / 100, animationPlayState: playing ? "running" : "paused" }} aria-hidden="true" />
        <div className="effect-stage-copy"><h2>{effect.name}</h2></div>
        <button type="button" className="preview-play" aria-label={playing ? "Pause effect preview" : "Play effect preview"} onClick={() => setPlaying(!playing)} disabled={effect.id === "solid"}>
          {playing ? <Pause size={16} /> : <Play size={16} />}
        </button>
      </TabsContent>)}
      <TabsList className="effect-options" aria-label="Choose a preview effect">
        {effects.map((effect) => <TabsTrigger className="effect-option" key={effect.id} value={effect.id}>
          <span className="effect-swatch" style={{ background: effect.background }} aria-hidden="true">{selected === effect.id && <span className="effect-selected"><Check size={12} /></span>}</span><span>{effect.name}</span>
        </TabsTrigger>)}
      </TabsList>
    </Tabs>
    <div className="preview-brightness"><Sun size={16} /><span id="preview-brightness-label">Brightness</span><Slider aria-labelledby="preview-brightness-label" min={0} max={100} step={1} value={[brightness]} onValueChange={([value]) => setBrightness(value ?? 0)} /><output>{brightness}%</output></div>
  </div>
}
