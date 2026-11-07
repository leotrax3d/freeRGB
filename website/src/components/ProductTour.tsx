import { useState } from "react"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs"
import lighting from "../../../docs/screenshots/lighting.png"
import layout from "../../../docs/screenshots/layout.png"
import cooling from "../../../docs/screenshots/cooling.png"

const screens = [
  { id: "lighting", label: "Lighting", image: lighting, text: "11 built-in effects, adjustable speed and colour, and up to eight custom layers.", alt: "freeRGB Lighting screen with effect swatches and controls for rainbow wave speed and brightness" },
  { id: "layout", label: "Device layout", image: layout, text: "Arrange devices on a shared effect canvas.", alt: "freeRGB Layout screen with keyboard, light strip and fan hub regions on a shared effect canvas" },
  { id: "cooling", label: "Cooling", image: cooling, text: "Set temperature-based fan and pump curves with a critical-temperature fallback.", alt: "freeRGB Cooling screen showing fan controls and temperature-based fan curves" },
] as const

export function ProductTour() {
  const [screen, setScreen] = useState("lighting")
  return <section id="demo" className="product-tour view-section">
    <header className="tour-heading"><h2>Explore freeRGB</h2></header>
    <Tabs value={screen} onValueChange={setScreen}>
      <TabsList className="tour-tabs" aria-label="Explore the freeRGB interface">{screens.map((item) => <TabsTrigger className="tour-tab" key={item.id} value={item.id}>{item.label}</TabsTrigger>)}</TabsList>
      {screens.map((item) => <TabsContent className="tour-content" key={item.id} value={item.id}><div className="tour-caption"><p>{item.text}</p></div><div className="app-screenshot"><img src={item.image} width="1440" height="1000" alt={item.alt} loading="lazy" /></div></TabsContent>)}
    </Tabs>
  </section>
}
