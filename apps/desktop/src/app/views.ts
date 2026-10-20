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
  type LucideIcon,
} from "lucide-react";

export type ViewId = "lighting" | "devices" | "layout" | "profiles" | "cooling" | "macros" | "monitoring" | "settings";

export interface ViewDef {
  id: ViewId;
  label: string;
  icon: LucideIcon;
  /** Shown in the sidebar footer group instead of the main list. */
  footer?: boolean;
  component: LazyExoticComponent<ComponentType>;
  /** Migration step that delivers the view; undefined once it is available. */
  planned?: string;
}

// A typed registry instead of a router: few top-level views, all lazy-loaded.
export const VIEWS: readonly ViewDef[] = [
  { id: "lighting", label: "Lighting", icon: Sparkles, component: lazy(() => import("@/features/lighting/LightingView")) },
  { id: "devices", label: "Devices", icon: Cpu, component: lazy(() => import("@/features/devices/DevicesView")) },
  { id: "layout", label: "Layout", icon: LayoutDashboard, component: lazy(() => import("@/features/layout/LayoutView")) },
  { id: "profiles", label: "Profiles", icon: Layers, component: lazy(() => import("@/features/profiles/ProfilesView")) },
  { id: "cooling", label: "Cooling", icon: Fan, component: lazy(() => import("@/features/cooling/CoolingView")) },
  { id: "macros", label: "Macros", icon: Keyboard, component: lazy(() => import("@/features/macros/MacrosView")) },
  { id: "monitoring", label: "Monitoring", icon: Activity, component: lazy(() => import("@/features/monitoring/MonitoringView")) },
  { id: "settings", label: "Settings", icon: Settings, footer: true, component: lazy(() => import("@/features/settings/SettingsView")) },
];
