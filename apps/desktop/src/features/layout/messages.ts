import { defineMessages } from "@/lib/i18n";

const en = {
  "layout.title": "Layout",
  "layout.description": "Where each device sits on the effect canvas. Effects are painted onto this canvas and every device shows its part.",
  "layout.empty.title": "No devices to place",
  "layout.empty.text": "Devices appear here as soon as the service finds them.",
  "layout.help": "Drag to move, drag the corner to resize. Snaps to the 5 % grid when you let go — hold Alt for free placement. The tinted background only suggests the current effect.",
  "layout.aside": "Device placement",
  "layout.leds": "{n} LEDs · {vendor}",
  "layout.whole": "Use whole canvas",
  "layout.left": "Left",
  "layout.top": "Top",
  "layout.width": "Width",
  "layout.height": "Height",
} as const;

const de: Record<keyof typeof en, string> = {
  "layout.title": "Anordnung",
  "layout.description": "Wo jedes Gerät auf der Effektfläche sitzt. Effekte werden auf diese Fläche gemalt, und jedes Gerät zeigt seinen Ausschnitt.",
  "layout.empty.title": "Keine Geräte zum Platzieren",
  "layout.empty.text": "Geräte erscheinen hier, sobald der Dienst sie findet.",
  "layout.help": "Zum Verschieben ziehen, an der Ecke ziehen zum Skalieren. Rastet beim Loslassen am 5-%-Raster ein – mit Alt frei platzieren. Der getönte Hintergrund deutet nur den aktuellen Effekt an.",
  "layout.aside": "Geräteplatzierung",
  "layout.leds": "{n} LEDs · {vendor}",
  "layout.whole": "Ganze Fläche nutzen",
  "layout.left": "Links",
  "layout.top": "Oben",
  "layout.width": "Breite",
  "layout.height": "Höhe",
};

export type LayoutKey = keyof typeof en;
export const { t } = defineMessages("layout", { en, de });
