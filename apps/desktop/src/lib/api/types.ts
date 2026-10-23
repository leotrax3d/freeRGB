// TypeScript mirror of the daemon's serde types (docs/LOCAL_API.md).
// Field names and tags must match Rust exactly; fixtures in types.test.ts guard this.

import type { CoolingStatus, FanCurve, MacroDefinition, MacroStatus, SensorGraph, SensorValues } from "./control";

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** Normalised placement on the effect canvas. */
export interface Region {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type BlendMode = "normal" | "add" | "multiply" | "screen" | "max";

export interface Layer {
  effect: Exclude<EffectSpec, { type: "layers" }>;
  blend: BlendMode;
  opacity: number;
  region?: Region | null;
}

export type EffectSpec =
  | { type: "solid"; color: Rgb }
  | { type: "rainbow_wave"; speed: number; scale: number }
  | { type: "breathing"; color: Rgb; period_ms: number }
  | { type: "gradient"; from: Rgb; to: Rgb; angle_deg: number }
  | { type: "color_cycle"; period_ms: number }
  | { type: "radial_rainbow"; speed: number; scale: number }
  | { type: "scanner"; color: Rgb; background: Rgb; period_ms: number; width: number }
  | { type: "twinkle"; color: Rgb; background: Rgb; density: number; period_ms: number }
  | { type: "audio_pulse"; color: Rgb; background: Rgb; sensitivity: number }
  | { type: "audio_spectrum"; low: Rgb; high: Rgb; background: Rgb }
  | { type: "screen_ambience"; saturation: number }
  | { type: "layers"; layers: Layer[] };

export type EffectName = EffectSpec["type"];

export type DeviceKind =
  | "Keyboard"
  | "Mouse"
  | "Headset"
  | "LightingController"
  | "FanHub"
  | "Cooler"
  | "Memory"
  | "Virtual"
  | "Other";

export type DeviceStatus = "Connected" | "Disconnected" | { Faulted: string };

export interface DeviceInfo {
  id: string;
  name: string;
  vendor: string;
  kind: DeviceKind;
  usb_vid: number | null;
  usb_pid: number | null;
  serial: string | null;
  firmware: string | null;
}

export interface SensorReading {
  id: string;
  name: string;
  unit: string;
  value: number | null;
  age_ms: number;
}

export interface DeviceSummary {
  info: DeviceInfo;
  status: DeviceStatus;
  brightness: number;
  region: Region;
  leds: number;
  frames: number;
  dropped: number;
  last_error: string | null;
  sensors: SensorReading[];
}

export interface Diagnostics {
  uptime_s: number;
  ticks: number;
  target_fps: number;
  measured_fps: number;
  effect: EffectSpec;
  brightness: number;
  devices: number;
  conflicts: string[];
  audio: string | null;
  screen: string | null;
}

/** Emitted by the desktop backend, not by the daemon. */
export interface Connection {
  connected: boolean;
  error: string | null;
  daemon_version: string | null;
  api_version: number | null;
}

/** Error shape returned by the backend's `rpc` command. */
export type CallError =
  | { kind: "forbidden"; method: string }
  | { kind: "rpc"; code: number; message: string }
  | { kind: "transport"; message: string }
  | { kind: "superseded" };

/** State slices pushed by the backend poller. */
export interface Slices {
  monitoring_sensors: MonitoringSensor[] | null;
  providers: ProviderSummary[] | null;
  cooling_channels: CoolingChannel[] | null;
  cooling_session: CoolingSession | null;
  cooling_bindings: ChannelBindingsView | null;
  cooling_background: CoolingBackground | null;
  cooling_support: CoolingSupportView | null;
  cooling_profiles: CoolingProfiles | null;
  keyboard_reactive: KeyboardReactive | null;
  connection: Connection;
  devices: DeviceSummary[] | null;
  diagnostics: Diagnostics | null;
  effect: EffectSpec | null;
  /** null while the daemon runs without storage (`--no-state`). */
  profiles: string[] | null;
  custom_effects: string[] | null;
  plugins: PluginSummary[] | null;
  curves: Record<string, FanCurve> | null;
  macros: Record<string, MacroDefinition> | null;
  macro_status: MacroStatus | null;
  sensor_values: SensorValues | null;
  sensor_graph: SensorGraph | null;
  /** Native Celsius source key → display label. */
  sensor_sources: Record<string, string> | null;
  cooling_status: CoolingStatus | null;
  /** Device setup and hardware modules (`modules.status`); null when disabled. */
  modules_setup: ModulesStatus | null;
  onboarding: OnboardingStatus | null;
}

export interface MonitoringSensor {
  provider: string;
  device_id: string;
  device_name: string;
  sensor: SensorReading;
}
export interface ProviderSummary {
  id: string;
  name: string;
  status: string;
  message: string | null;
  age_ms: number | null;
}
/** Protected inventory: what a control can reach (DECISIONS.md #14). */
export interface CoolingSupport {
  tier: "verified" | "lhm_supported";
  register_readback: boolean;
  software_mode: boolean;
  held_full: boolean;
}
export type SupportTier = "verified" | "lhm_supported" | "unrestricted" | "read_only";
export interface CoolingChannel {
  provider: string;
  id: string;
  device_id: string;
  name: string;
  kind: "fan" | "control";
  reading: SensorReading;
  control_available: boolean;
  sensor_id?: string;
  hardware_name?: string;
  support?: CoolingSupport | null;
}
export interface CoolingSession {
  available: boolean;
  job_id: number | null;
  state: string;
  report: unknown;
  error: string | null;
  tier?: SupportTier | null;
}
/** Saved background curves (run by the protected service from boot). */
export interface BackgroundChannel {
  control_id: string;
  temperature_id: string;
  rpm_id: string;
  curve: import("./control").FanCurve;
  confirmed_fan: boolean;
}
export interface BackgroundServiceChannel {
  control_id: string;
  tier: SupportTier;
  state: string;
  duty_percent: number;
  fault: string | null;
  warning: string | null;
}
export interface CoolingBackground {
  available: boolean;
  saved: { channels: BackgroundChannel[] };
  apply: { state: "idle" | "applying" | "applied" | "failed"; error: string | null };
  service: { version: 1; error: string | null; updated_utc: string; channels: BackgroundServiceChannel[] } | null;
}
/** Per-channel evidence: guided test and unrestricted unlock (bound to board/BIOS/channel). */
export interface ChannelEvidenceView {
  test: "none" | "passed" | "failed" | "stale";
  tested: { duty_percent: number; rpm: number }[] | null;
  unlock: "none" | "stale" | "disabled" | "unlocked" | "unlocked_pump";
}
export interface CoolingSupportView {
  unrestricted_enabled: boolean;
  board_identity_available: boolean;
  channels: Record<string, ChannelEvidenceView>;
  error: string | null;
}
/** Named background channel sets. */
export interface CoolingProfiles {
  profiles: { name: string; channels: number }[];
  active: string | null;
  error: string | null;
}
export interface ReactiveConfig {
  version: number;
  enabled: boolean;
  color: Rgb;
  fade_ms: number;
  iso: boolean;
  devices: string[];
}
export interface ChannelBinding {
  provider: string;
  device_id: string;
  control_id: string;
  tachometer_id: string | null;
  header: string;
  role: "fan" | "pump";
  electrical_mode: "unknown" | "pwm" | "dc";
}
export interface ChannelBindingsView {
  config: { version: number; bindings: ChannelBinding[] };
  assignments: { control_id: string; state: string }[];
  error: string | null;
  persistence_enabled: boolean;
}
export interface KeyboardReactive {
  config: ReactiveConfig;
  status: { state: string; error: string | null };
  devices: { id: string; name: string; connected: boolean; keys: Record<string, number> }[];
}

export interface PluginSummary {
  id: string;
  name: string;
  access: "virtual" | "hardware";
  enabled: boolean;
  allowed: boolean;
  status: DeviceStatus | null;
  last_error: string | null;
  frames: number;
}

export type SliceName = keyof Slices;
export type OptionalSlice = Exclude<SliceName, "connection" | "devices" | "diagnostics" | "effect" | "modules_setup" | "onboarding">;

/** First-run wizard and fan-control safety acknowledgement (docs/SETUP_WIZARD.md). */
export interface OnboardingStatus {
  completed: boolean;
  safety_text_version: number;
  safety_acknowledged: boolean;
  safety_acknowledged_unix: number | null;
}

export function deviceStatus(status: DeviceStatus): "connected" | "disconnected" | "faulted" {
  if (status === "Connected") return "connected";
  if (status === "Disconnected") return "disconnected";
  return "faulted";
}

export function faultMessage(status: DeviceStatus): string | null {
  return typeof status === "object" ? status.Faulted : null;
}

/** Phases a setup card can be in; mirrors `freergb_modules::setup::Phase`. */
export type SetupPhase =
  | "detected"
  | "waiting"
  | "downloading"
  | "verifying"
  | "installing"
  | "initializing"
  | "ready"
  | "failed"
  | "not_connected"
  | "unsupported";

export type ModuleDeviceKind =
  | "keyboard" | "mouse" | "mousemat" | "headset" | "headset_stand" | "keypad"
  | "lighting_controller" | "speaker" | "accessory" | "laptop" | "other";

export interface SetupCard {
  key: string;
  name: string;
  vendor: string;
  kind: ModuleDeviceKind;
  phase: SetupPhase;
  module: string | null;
  download_size: number | null;
  progress: { done: number; total: number } | null;
  error: { code: string; message: string } | null;
  configured: boolean;
  dismissed: boolean;
  builtin: boolean;
  installed: boolean;
  connected: boolean;
  engine_id: string | null;
  technical: string;
}

export interface InstalledModule {
  id: string;
  version: string;
  previous: string | null;
  last_good: string | null;
  family: string;
  license: string;
}

export interface ModulesStatus {
  first_run: boolean;
  cards: SetupCard[];
  catalog: { state: "none" | "loading" | "ready" | "cached" | "error"; source: string; message: string | null; sequence: number | null };
  job: { running: boolean; cancelled: boolean; bytes_done: number; bytes_total: number; modules: string[] };
  installed: InstalledModule[];
  trusted_release_key: boolean;
  light_test_remaining_s: number | null;
}


/** Release update state from the desktop updater (same checks as Classic). */
export interface UpdateInfo {
  current: string;
  available: string | null;
  size: number | null;
  downloaded: boolean;
}
