import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import {
  Activity,
  Cpu,
  Fan,
  Keyboard,
  LayoutDashboard,
  Layers,
  Settings,
  Sparkles,
  Store,
  type LucideIcon,
} from "lucide-react";

export type ViewId = "marketplace" | "lighting" | "devices" | "layout" | "profiles" | "cooling" | "macros" | "monitoring" | "settings";

export type ViewGroup = "light" | "hardware" | "automation" | "footer";

export interface ViewDef {
  id: ViewId;
  label: string;
  icon: LucideIcon;
  group: ViewGroup;
  /** Short hint shown in the command palette. */
  hint: string;
  component: LazyExoticComponent<ComponentType>;
  load: () => Promise<unknown>;
}

export const GROUP_LABELS: Record<Exclude<ViewGroup, "footer">, string> = {
  light: "Light",
  hardware: "Hardware",
  automation: "Automation",
};

function view(def: Omit<ViewDef, "component" | "load">, load: () => Promise<{ default: ComponentType }>): ViewDef {
  return { ...def, component: lazy(load), load };
}

// A typed registry instead of a router: few top-level views, all lazy-loaded.
export const VIEWS: readonly ViewDef[] = [
  view({ id: "marketplace", label: "Marketplace", icon: Store, group: "light", hint: "Discover and manage official effects" }, () => import("@/features/marketplace/MarketplaceView")),
  view({ id: "lighting", label: "Lighting", icon: Sparkles, group: "light", hint: "Effects, colours, brightness" }, () => import("@/features/lighting/LightingView")),
  view({ id: "layout", label: "Layout", icon: LayoutDashboard, group: "light", hint: "Place devices on the canvas" }, () => import("@/features/layout/LayoutView")),
  view({ id: "profiles", label: "Profiles", icon: Layers, group: "light", hint: "Save and switch lighting" }, () => import("@/features/profiles/ProfilesView")),
  view({ id: "devices", label: "Devices", icon: Cpu, group: "hardware", hint: "Status and per-device brightness" }, () => import("@/features/devices/DevicesView")),
  view({ id: "cooling", label: "Cooling", icon: Fan, group: "hardware", hint: "Fan curves and temperature sources" }, () => import("@/features/cooling/CoolingView")),
  view({ id: "monitoring", label: "Monitoring", icon: Activity, group: "hardware", hint: "Sensors and diagnostics" }, () => import("@/features/monitoring/MonitoringView")),
  view({ id: "macros", label: "Macros", icon: Keyboard, group: "automation", hint: "Step sequences" }, () => import("@/features/macros/MacrosView")),
  view({ id: "settings", label: "Settings", icon: Settings, group: "footer", hint: "Plugins, updates, about" }, () => import("@/features/settings/SettingsView")),
];

/** Loads every view chunk once the app is idle, so switching never shows a blank panel. */
export function preloadViews() {
  const run = () => VIEWS.forEach((v) => void v.load());
  if ("requestIdleCallback" in window) requestIdleCallback(run, { timeout: 2000 });
  else setTimeout(run, 500);
}
