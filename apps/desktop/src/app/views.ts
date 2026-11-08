import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import type { MessageKey } from "@/lib/i18n";
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

export type ViewId = "marketplace" | "lighting" | "devices" | "headers" | "layout" | "profiles" | "cooling" | "macros" | "monitoring" | "settings";

export type ViewGroup = "light" | "hardware" | "automation" | "footer";

export interface ViewDef {
  id: ViewId;
  /** Message keys: resolved with `t()` at render time so the language can change. */
  label: MessageKey;
  icon: LucideIcon;
  group: ViewGroup;
  /** Short hint shown in the command palette. */
  hint: MessageKey;
  component: LazyExoticComponent<ComponentType>;
  load: () => Promise<unknown>;
}

export const GROUP_LABELS: Record<Exclude<ViewGroup, "footer">, MessageKey> = {
  light: "nav.group.light",
  hardware: "nav.group.hardware",
  automation: "nav.group.automation",
};

function view(def: Omit<ViewDef, "component" | "load">, load: () => Promise<{ default: ComponentType }>): ViewDef {
  return { ...def, component: lazy(load), load };
}

// A typed registry instead of a router: few top-level views, all lazy-loaded.
export const VIEWS: readonly ViewDef[] = [
  view({ id: "marketplace", label: "nav.marketplace", icon: Store, group: "light", hint: "nav.marketplace.hint" }, () => import("@/features/marketplace/MarketplaceView")),
  view({ id: "lighting", label: "nav.lighting", icon: Sparkles, group: "light", hint: "nav.lighting.hint" }, () => import("@/features/lighting/LightingView")),
  view({ id: "layout", label: "nav.layout", icon: LayoutDashboard, group: "light", hint: "nav.layout.hint" }, () => import("@/features/layout/LayoutView")),
  view({ id: "profiles", label: "nav.profiles", icon: Layers, group: "light", hint: "nav.profiles.hint" }, () => import("@/features/profiles/ProfilesView")),
  view({ id: "devices", label: "nav.devices", icon: Cpu, group: "hardware", hint: "nav.devices.hint" }, () => import("@/features/devices/DevicesView")),
  view({ id: "headers", label: "mainboardRgb.headers", icon: Cpu, group: "hardware", hint: "mainboardRgb.hint" }, () => import("@/features/devices/HeadersView")),
  view({ id: "cooling", label: "nav.cooling", icon: Fan, group: "hardware", hint: "nav.cooling.hint" }, () => import("@/features/cooling/CoolingView")),
  view({ id: "monitoring", label: "nav.monitoring", icon: Activity, group: "hardware", hint: "nav.monitoring.hint" }, () => import("@/features/monitoring/MonitoringView")),
  view({ id: "macros", label: "nav.macros", icon: Keyboard, group: "automation", hint: "nav.macros.hint" }, () => import("@/features/macros/MacrosView")),
  view({ id: "settings", label: "nav.settings", icon: Settings, group: "footer", hint: "nav.settings.hint" }, () => import("@/features/settings/SettingsView")),
];

/** Loads every view chunk once the app is idle, so switching never shows a blank panel. */
export function preloadViews() {
  const run = () => VIEWS.forEach((v) => void v.load());
  if ("requestIdleCallback" in window) requestIdleCallback(run, { timeout: 2000 });
  else setTimeout(run, 500);
}
