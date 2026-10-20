// MOCK DATA — only used when the UI runs in a plain browser without Tauri
// (design work via `npm run dev`). The desktop app never loads this module
// unless it is opened outside Tauri. Values are illustrative, not measurements.

import type { SliceListener, Transport } from "../api/transport";
import { RpcError } from "../api/transport";
import { DEFAULT_CURVE, type FanCurve, type MacroDefinition, type SensorGraph } from "../api/control";
import type { DeviceSummary, EffectSpec, Slices } from "../api/types";
import effectDefaults from "../api/fixtures/effects.json";

let devices: DeviceSummary[] = [
  {
    info: {
      id: "mock:keyboard",
      name: "K70 CORE (mock)",
      vendor: "Corsair",
      kind: "Keyboard",
      usb_vid: null,
      usb_pid: null,
      serial: null,
      firmware: null,
    },
    status: "Connected",
    brightness: 1,
    region: { x: 0, y: 0.55, w: 0.6, h: 0.45 },
    leds: 123,
    frames: 0,
    dropped: 0,
    last_error: null,
    sensors: [],
  },
  {
    info: {
      id: "mock:link",
      name: "iCUE LINK hub (mock)",
      vendor: "Corsair",
      kind: "FanHub",
      usb_vid: null,
      usb_pid: null,
      serial: null,
      firmware: null,
    },
    status: "Connected",
    brightness: 0.8,
    region: { x: 0.65, y: 0, w: 0.35, h: 1 },
    leds: 64,
    frames: 0,
    dropped: 3,
    last_error: null,
    sensors: [
      { id: "fan:1", name: "Fan 1 (mock)", unit: "rpm", value: 1180, age_ms: 400 },
      { id: "temperature:4", name: "Coolant (mock)", unit: "°C", value: 31.5, age_ms: 400 },
    ],
  },
  {
    info: {
      id: "mock:virtual",
      name: "Virtual strip (mock)",
      vendor: "freeRGB",
      kind: "Virtual",
      usb_vid: null,
      usb_pid: null,
      serial: null,
      firmware: null,
    },
    status: { Faulted: "Mock fault: device stopped answering" },
    brightness: 1,
    region: { x: 0, y: 0, w: 0.6, h: 0.5 },
    leds: 30,
    frames: 0,
    dropped: 0,
    last_error: "Mock fault: device stopped answering",
    sensors: [],
  },
];

export function mockTransport(): Transport {
  const listeners = new Set<SliceListener>();
  let effect = (effectDefaults as Record<string, EffectSpec>).rainbow_wave!;
  let brightness = 0.85;
  const started = Date.now();
  const profiles = new Map<string, { effect: EffectSpec; brightness: number }>([
    ["Evening", { effect: (effectDefaults as Record<string, EffectSpec>).breathing!, brightness: 0.4 }],
    ["Gaming", { effect: (effectDefaults as Record<string, EffectSpec>).radial_rainbow!, brightness: 1 }],
  ]);
  const custom = new Map<string, EffectSpec>();
  const sorted = (m: Map<string, unknown>) => [...m.keys()].sort();
  const curves: Record<string, FanCurve> = { Quiet: DEFAULT_CURVE };
  const macros: Record<string, MacroDefinition> = {
    Flash: { repeat: 2, steps: [{ type: "color", color: { r: 255, g: 0, b: 0 } }, { type: "delay", ms: 300 }, { type: "brightness", value: 0.7 }] },
  };
  let graph: SensorGraph = { version: 1, nodes: { coolant: { type: "input", source: "[\"mock:link\",\"temperature:4\"]" } } };

  const diagnostics = () => ({
    uptime_s: (Date.now() - started) / 1000,
    ticks: 0,
    target_fps: 60,
    measured_fps: 59.8,
    effect,
    brightness,
    devices: devices.length,
    conflicts: [],
    audio: null,
    screen: null,
  });
  const slices = (): Partial<Slices> => ({
    connection: { connected: true, error: null, daemon_version: "mock", api_version: 1 },
    devices,
    diagnostics: diagnostics(),
    effect,
    profiles: sorted(profiles),
    custom_effects: sorted(custom),
    plugins: [],
    curves: { ...curves },
    macros: { ...macros },
    macro_status: { running: null, input_enabled: false, error: null },
    cooling_status: { hardware_control: false, reason: "Mock data: nothing is written.", error: null },
    sensor_graph: graph,
    sensor_sources: { "[\"mock:link\",\"temperature:4\"]": "iCUE LINK hub (mock) / Coolant (mock)" },
    sensor_values: {
      now_ms: Date.now() - started,
      values: Object.fromEntries(Object.keys(graph.nodes).map((n) => [n, { value: 31.5, sampled_at_ms: 0, error: null }])),
    },
  });
  const emit = (slice: keyof Slices) => {
    const value = slices()[slice];
    listeners.forEach((l) => l(slice, value));
  };

  return {
    kind: "mock",
    async rpc<T>(method: string, params?: unknown): Promise<T> {
      const p = (params ?? {}) as Record<string, unknown>;
      switch (method) {
        case "effects.set":
          effect = params as EffectSpec;
          emit("effect");
          return true as T;
        case "brightness.set":
          brightness = p.value as number;
          emit("diagnostics");
          return true as T;
        case "devices.set_brightness": {
          const device = devices.find((d) => d.info.id === p.id);
          if (device) devices = devices.map((d) => (d === device ? { ...d, brightness: p.value as number } : d));
          emit("devices");
          return true as T;
        }
        case "devices.set_region": {
          const device = devices.find((d) => d.info.id === p.id);
          if (!device) throw new RpcError({ kind: "rpc", code: -32004, message: `no device with id ${String(p.id)}` });
          devices = devices.map((d) => (d === device ? { ...d, region: { x: p.x as number, y: p.y as number, w: p.w as number, h: p.h as number } } : d));
          emit("devices");
          return true as T;
        }
        case "profiles.save":
          profiles.set(p.name as string, { effect, brightness });
          emit("profiles");
          return true as T;
        case "profiles.load": {
          const profile = profiles.get(p.name as string);
          if (!profile) throw new RpcError({ kind: "rpc", code: -32004, message: "profile not found" });
          ({ effect, brightness } = profile);
          emit("effect");
          emit("diagnostics");
          return true as T;
        }
        case "profiles.delete":
          profiles.delete(p.name as string);
          emit("profiles");
          return true as T;
        case "custom_effects.save":
          custom.set(p.name as string, p.effect as EffectSpec);
          emit("custom_effects");
          return true as T;
        case "custom_effects.get":
          return custom.get(p.name as string) as T;
        case "custom_effects.delete":
          custom.delete(p.name as string);
          emit("custom_effects");
          return true as T;
        case "cooling.preview": {
          const c = p.curve as FanCurve;
          const t = p.temperature_c as number;
          const pts = c.points;
          let duty = t >= c.critical_c ? 100 : pts[pts.length - 1]!.duty_percent;
          if (t <= pts[0]!.temperature_c) duty = pts[0]!.duty_percent;
          for (let i = 1; i < pts.length && t < c.critical_c; i++) {
            const a = pts[i - 1]!, b = pts[i]!;
            if (t > a.temperature_c && t <= b.temperature_c) {
              duty = a.duty_percent + ((t - a.temperature_c) / (b.temperature_c - a.temperature_c)) * (b.duty_percent - a.duty_percent);
              break;
            }
          }
          return { duty_percent: duty, simulated: true } as T;
        }
        case "cooling.curves.save":
          curves[p.name as string] = p.curve as FanCurve;
          emit("curves");
          return true as T;
        case "cooling.curves.delete":
          delete curves[p.name as string];
          emit("curves");
          return true as T;
        case "macros.save":
          macros[p.name as string] = p.definition as MacroDefinition;
          emit("macros");
          return true as T;
        case "macros.delete":
          delete macros[p.name as string];
          emit("macros");
          return true as T;
        case "macros.run":
        case "macros.cancel":
          return true as T;
        case "cooling.sensors.save":
          graph = (p.graph as SensorGraph);
          emit("sensor_graph");
          return true as T;
        case "effects.default":
          return (effectDefaults as Record<string, unknown>)[p.name as string] as T;
        default:
          throw new RpcError({ kind: "rpc", code: -32601, message: `${method} is not mocked` });
      }
    },
    snapshot: async () => slices(),
    async subscribe(listener) {
      listeners.add(listener);
      const timer = setInterval(() => emit("diagnostics"), 1000);
      return () => {
        clearInterval(timer);
        listeners.delete(listener);
      };
    },
    wantSlices: async () => {},
    startService: async () => {},
    exportDiagnostics: async () => {
      throw new Error("Not available with mock data");
    },
  };
}
