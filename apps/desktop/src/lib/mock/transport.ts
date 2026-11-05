// MOCK DATA — only used when the UI runs in a plain browser without Tauri
// (design work via `npm run dev`). The desktop app never loads this module
// unless it is opened outside Tauri. Values are illustrative, not measurements.

import type { SliceListener, Transport } from "../api/transport";
import { RpcError } from "../api/transport";
import { DEFAULT_CURVE, type CurveSet, type FanCurve, type MacroDefinition, type SensorGraph } from "../api/control";
import type { ChannelBinding, OnboardingStatus, BackgroundChannel, ChannelEvidenceView, CoolingBackground, CoolingChannel, CoolingSession, CoolingSupportView, DeviceSummary, EffectSpec, MonitoringSensor, Slices } from "../api/types";
import effectDefaults from "../api/fixtures/effects.json";
import effectCatalog from "../../../../../effects/catalog.json";
import { mockSetup } from "./setup";

const initialDevices: DeviceSummary[] = [
  {
    info: {
      id: "mock:keyboard",
      name: "RGB Keyboard (virtual)",
      vendor: "Demo Hardware",
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
      name: "RGB Fan Hub (virtual)",
      vendor: "Demo Hardware",
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

function requiresCapture(effect: EffectSpec): boolean {
  return ["audio_pulse", "audio_spectrum", "screen_ambience"].includes(effect.type)
    || (effect.type === "layers" && effect.layers.some((layer) => requiresCapture(layer.effect)));
}

export function mockTransport(): Transport {
  let devices = structuredClone(initialDevices);
  const listeners = new Set<SliceListener>();
  const setup = mockSetup(() => emit("modules_setup"));
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
  let curveSet: CurveSet = { nodes: { floor: { type: "constant", percent: 35 }, case: { type: "follow", source: "floor", scale: 1, offset_percent: 10 } } };
  let graph: SensorGraph = { version: 1, nodes: { coolant: { type: "input", source: "[\"mock:link\",\"temperature:4\"]" } } };

  // Mock motherboard fan control (nothing is written): one verified Nuvoton
  // header, one LHM-supported ITE header, one header owned by another tool.
  const reading = (id: string, name: string, unit: string, value: number) => ({ id, name, unit, value, age_ms: 300 });
  const control = (hw: string, index: number, name: string, tier: "verified" | "lhm_supported", softwareMode = false): CoolingChannel => ({
    provider: "mainboard.lhm", id: JSON.stringify([hw, `${hw}/control/${index}`]), device_id: `mainboard.lhm:${hw}`, name, kind: "control",
    reading: reading(`${hw}/control/${index}`, name, "%", 55), control_available: true, sensor_id: `${hw}/control/${index}`, hardware_name: hw,
    support: { tier, register_readback: tier === "verified", software_mode: softwareMode, held_full: false },
  });
  const fan = (hw: string, index: number, name: string, rpm: number): CoolingChannel => ({
    provider: "mainboard.lhm", id: JSON.stringify([hw, `${hw}/fan/${index}`]), device_id: `mainboard.lhm:${hw}`, name, kind: "fan",
    reading: reading(`${hw}/fan/${index}`, name, "RPM", rpm), control_available: false,
  });
  const coolingChannels: CoolingChannel[] = [
    control("/lpc/nct6701d/0", 2, "CHA_FAN2 Control (mock)", "verified"),
    fan("/lpc/nct6701d/0", 2, "CHA_FAN2 (mock)", 1240),
    control("/lpc/it8689e/0", 1, "SYS_FAN1 Control (mock)", "lhm_supported"),
    fan("/lpc/it8689e/0", 1, "SYS_FAN1 (mock)", 980),
    control("/lpc/it8689e/0", 3, "SYS_FAN3 Control (mock)", "lhm_supported", true),
    // Demo: more headers so several live channels can run at once.
    control("/lpc/nct6701d/0", 0, "CPU_FAN Control (mock)", "verified"),
    fan("/lpc/nct6701d/0", 0, "CPU_FAN (mock)", 1105),
    control("/lpc/nct6701d/0", 1, "CHA_FAN1 Control (mock)", "verified"),
    fan("/lpc/nct6701d/0", 1, "CHA_FAN1 (mock)", 870),
    control("/lpc/nct6701d/0", 4, "AIO_PUMP Control (mock)", "verified"),
    fan("/lpc/nct6701d/0", 4, "AIO_PUMP (mock)", 2650),
  ];
  const boardSensors: MonitoringSensor[] = [
    { provider: "mainboard.lhm", device_id: "mainboard.lhm:/lpc/nct6701d/0", device_name: "Nuvoton NCT6701D (mock)", sensor: reading("/lpc/nct6701d/0/temperature/1", "CPU (mock)", "°C", 46) },
    { provider: "mainboard.lhm", device_id: "mainboard.lhm:/lpc/it8689e/0", device_name: "ITE IT8689E (mock)", sensor: reading("/lpc/it8689e/0/temperature/0", "System (mock)", "°C", 38) },
  ];
  let session: CoolingSession = { available: true, job_id: null, state: "idle", report: null, error: null, tier: null };
  const mockJobs = new Map<number, CoolingSession>();
  const setJob = (job: CoolingSession) => { mockJobs.set(job.job_id!, job); session = job; emit("cooling_session"); };
  let unrestricted = false;
  let onboarding: OnboardingStatus = { completed: false, safety_text_version: 1, safety_acknowledged: false, safety_acknowledged_unix: null };
  // Demo: three identified headers (two fans, one pump) and two live channels.
  const demoBinding = (index: number, header: string, role: "fan" | "pump"): ChannelBinding => ({
    provider: "mainboard.lhm", device_id: "mainboard.lhm:/lpc/nct6701d/0", control_id: JSON.stringify(["/lpc/nct6701d/0", `/lpc/nct6701d/0/control/${index}`]),
    tachometer_id: JSON.stringify(["/lpc/nct6701d/0", `/lpc/nct6701d/0/fan/${index}`]), header, role, electrical_mode: "unknown",
  });
  let mainboardRgb: { enabled: boolean; controllers: Record<string, { leds: number; name?: string | null }[]> } =
    { enabled: true, controllers: { "0b05:19af": [{ leds: 16, name: "Front fans" }, { leds: 0 }, { leds: 0 }, { leds: 0 }] } };
  let bindingsConfig: ChannelBinding[] = [demoBinding(0, "CPU_FAN", "fan"), demoBinding(1, "CHA_FAN1", "fan"), demoBinding(4, "AIO_PUMP", "pump")];
  for (const [job, index, mode] of [[1, 0, "curve"], [2, 1, "manual"]] as const) {
    mockJobs.set(job, { available: true, job_id: job, state: "executing", report: null, error: null, tier: "verified", mode, control_id: `/lpc/nct6701d/0/control/${index}` });
  }
  const evidence: Record<string, ChannelEvidenceView> = {
    "/lpc/nct6701d/0/control/2": { test: "passed", tested: [{ duty_percent: 100, rpm: 1710 }, { duty_percent: 70, rpm: 1330 }, { duty_percent: 50, rpm: 1010 }], unlock: "none", rpm_range: { min_rpm: 1010, max_rpm: 1710 } },
    "/lpc/it8689e/0/control/1": { test: "none", tested: null, unlock: "none" },
    "/lpc/it8689e/0/control/3": { test: "none", tested: null, unlock: "none" },
    "/lpc/nct6701d/0/control/0": { test: "passed", tested: [{ duty_percent: 100, rpm: 1900 }, { duty_percent: 70, rpm: 1400 }, { duty_percent: 50, rpm: 1050 }], unlock: "none", rpm_range: { min_rpm: 1050, max_rpm: 1900 } },
    "/lpc/nct6701d/0/control/1": { test: "passed", tested: [{ duty_percent: 100, rpm: 1500 }, { duty_percent: 70, rpm: 1100 }, { duty_percent: 50, rpm: 820 }], unlock: "none", rpm_range: { min_rpm: 820, max_rpm: 1500 } },
    "/lpc/nct6701d/0/control/4": { test: "none", tested: null, unlock: "none" },
  };
  let backgroundSaved: BackgroundChannel[] = [];
  let applyState: CoolingBackground["apply"] = { state: "idle", error: null };
  const coolingProfiles = new Map<string, BackgroundChannel[]>();
  let activeProfile: string | null = null;
  const tierOf = (id: string) => coolingChannels.find((c) => c.reading.id === id)?.support?.tier ?? "verified";
  const effectiveTier = (id: string) => (evidence[id]?.unlock.startsWith("unlocked") && unrestricted ? "unrestricted" : tierOf(id));
  const background = (): CoolingBackground => ({
    available: true, saved: { channels: backgroundSaved }, apply: applyState,
    service: { version: 1, error: null, updated_utc: new Date().toISOString(), channels: applyState.state === "applied" ? backgroundSaved.map((c) => ({ control_id: c.control_id, tier: effectiveTier(c.control_id), state: "running", duty_percent: Math.max(40, c.curve.minimum_percent), fault: null, warning: null })) : [] },
  });
  const supportView = (): CoolingSupportView => ({ unrestricted_enabled: unrestricted, board_identity_available: true, channels: evidence, error: null });
  const fail = (message: string): never => { throw new RpcError({ kind: "rpc", code: -32006, message }); };
  const coolingCheck = (id: string, minimum: number) => {
    if (!onboarding.safety_acknowledged) fail("Fan control is locked until the safety notice is acknowledged in the setup wizard");
    const c = coolingChannels.find((x) => x.reading.id === id);
    if (c?.support?.software_mode) fail("This channel is already in software mode (another tool controls it); freeRGB will not take it over");
    const tier = effectiveTier(id);
    if (tier === "unrestricted") return tier;
    if (tier === "lhm_supported" && evidence[id]?.test !== "passed" && minimum < 100) fail("Pass the guided channel test before running a reduced-duty curve on this LHM-supported channel");
    if (minimum < 40) fail("Native curve floor >=40%, critical <=100 C required");
    return tier;
  };
  const fakeApply = () => { applyState = { state: "applying", error: null }; emit("cooling_background"); setTimeout(() => { applyState = { state: "applied", error: null }; emit("cooling_background"); }, 1200); };

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
    modules_setup: setup.get(),
    onboarding,
    cooling_bindings: { config: { version: 1, bindings: bindingsConfig }, assignments: [], error: null, persistence_enabled: true },
    curves: { ...curves },
    macros: { ...macros },
    macro_status: { running: null, input_enabled: false, error: null },
    cooling_status: { hardware_control: false, reason: "Mock data: nothing is written.", error: null },
    sensor_graph: graph,
    cooling_channels: coolingChannels,
    monitoring_sensors: boardSensors,
    providers: [{ id: "mainboard.lhm", name: "LibreHardwareMonitor motherboard", status: "ready", message: "Mock data: nothing is written.", age_ms: 300 }],
    cooling_session: { ...session, jobs: [...mockJobs.values()].reverse() },
    cooling_background: background(),
    cooling_support: supportView(),
    cooling_profiles: { profiles: [...coolingProfiles].map(([name, channels]) => ({ name, channels: channels.length })), active: activeProfile, error: null },
    sensor_sources: { "[\"mock:link\",\"temperature:4\"]": "RGB Fan Hub (virtual) / Coolant (mock)" },
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
          if (requiresCapture(params as EffectSpec)) throw new RpcError({ kind: "rpc", code: -32601, message: "Audio and screen capture effects require the native application and are unavailable in this demo." });
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
          throw new RpcError({ kind: "rpc", code: -32601, message: "Macro execution is not simulated. The editor is available." });
        case "cooling.sensors.save":
          graph = (p.graph as SensorGraph);
          emit("sensor_graph");
          return true as T;
        case "cooling.sessions.start": {
          const id = p.control_id as string;
          const mode = p.mode as string;
          const minimum = mode === "curve" ? (p.curve as FanCurve).minimum_percent : mode === "manual" ? (p.duty_percent as number) : mode === "identify" ? 100 : 40;
          const tier = coolingCheck(id, mode === "curve" || mode === "manual" ? minimum : 100);
          const job = Math.max(0, ...mockJobs.keys()) + 1;
          const base: CoolingSession = { available: true, job_id: job, state: "executing", report: null, error: null, tier, mode, control_id: id };
          setJob(base);
          const finish = (report: Record<string, unknown>) => setJob({ ...mockJobs.get(job)!, state: "completed", report });
          if (mode === "identify") setTimeout(() => finish({ tier, note: "mock identification" }), 8000);
          if (mode === "channel_test") setTimeout(() => {
            evidence[id] = { ...evidence[id]!, test: "passed", tested: [{ duty_percent: 100, rpm: 1500 }, { duty_percent: 70, rpm: 1120 }, { duty_percent: 50, rpm: 830 }] };
            finish({ tier, note: "mock channel test" });
            emit("cooling_support");
          }, 3000);
          return { job_id: job } as T;
        }
        case "onboarding.get":
          return onboarding as T;
        case "onboarding.complete":
          onboarding = { ...onboarding, completed: true }; emit("onboarding");
          return true as T;
        case "onboarding.reset":
          onboarding = { ...onboarding, completed: false }; emit("onboarding");
          return true as T;
        case "safety.acknowledge":
          if (p.accepted !== true || p.version !== 1) fail("The current safety notice must be read and accepted");
          onboarding = { ...onboarding, safety_acknowledged: true, safety_acknowledged_unix: Math.floor(Date.now() / 1000) }; emit("onboarding");
          return true as T;
        case "safety.revoke":
          onboarding = { ...onboarding, safety_acknowledged: false, safety_acknowledged_unix: null }; emit("onboarding");
          return true as T;
        case "cooling.bindings.save":
          bindingsConfig = ((p.config as { bindings: ChannelBinding[] }).bindings); emit("cooling_bindings");
          return true as T;
        case "cooling.hwmon.get":
          return { started: true, board: { manufacturer: "ASUSTeK COMPUTER INC.", product: "TUF GAMING B650-PLUS", bios_version: "3024" }, paused: false, unrestricted: false, floor_percent: 40,
            chips: [{ id: "nct6799@platform/nct6775.656", name: "nct6799", read_only_hint: false,
              temps: [{ index: 1, label: "SYSTIN", celsius: 34 }, { index: 2, label: "CPUTIN", celsius: 46 }],
              fans: [{ index: 1, label: "Fan 1", rpm: 1180 }, { index: 2, label: "Fan 2", rpm: 960 }],
              pwms: [1, 2].map((i) => ({ channel_id: `nct6799@platform/nct6775.656/pwm${i}`, label: `PWM ${i}`, percent: 45, enable: 5, mode: 1, writable: true, paired_fan: i })) }],
            assignments: [], status: [] } as T;
        case "cooling.sensors.catalog":
          return [
            { id: "[\"mock:link\",\"temperature:4\"]", label: "RGB Fan Hub (virtual) / Coolant (mock)", provider: "device", unit: "°C", category: "liquid", value: 31.2, age_ms: 0, error: null },
            { id: "[\"mainboard.lhm:/amdcpu/0\",\"/amdcpu/0/temperature/2\"]", label: "AMD Ryzen (mock) / Core (Tctl/Tdie)", provider: "mainboard.lhm", unit: "°C", category: "cpu", value: 54.8, age_ms: 0, error: null },
            { id: "[\"mainboard.lhm:/gpu-nvidia/0\",\"/gpu-nvidia/0/temperature/2\"]", label: "GeForce (mock) / GPU Hot Spot", provider: "mainboard.lhm", unit: "°C", category: "gpu", value: 61.0, age_ms: 0, error: null },
            { id: "file:ambient", label: "File / ambient", provider: "file", unit: "°C", category: "file", value: null, age_ms: 0, error: "file not updated within timeout" },
          ] as T;
        case "cooling.file_sensors.get":
          return { sensors: { ambient: { path: "C:\\temps\\ambient.txt" } } } as T;
        case "cooling.file_sensors.save":
          return true as T;
        case "cooling.gpu.list":
          return { started: true, paused: false, backends: ["simulated"], fans: [{ fan: { id: "nvml:GPU-mock/fan0", vendor: "nvidia", gpu_name: "GeForce (mock)", index: 0, duty_percent: 35, controllable: true, reason: null, min_percent: 30 }, floor_percent: 40, assignment: null, applied_percent: null, fault: null }] } as T;
        case "cooling.gpu.set":
        case "cooling.gpu.release":
          return true as T;
        case "cooling.hwmon.set":
        case "cooling.hwmon.release":
          return true as T;
        case "mainboard_rgb.get":
          return { settings: mainboardRgb, max_headers: 4, max_leds: 512, fan_presets: [8, 12, 16, 18], vendor_software: [], error: null } as T;
        case "mainboard_rgb.save": {
          const next = p.settings as typeof mainboardRgb;
          if (Object.values(next.controllers).some((h) => h.length > 4 || h.some((x) => x.leds < 0 || x.leds > 512))) fail("ARGB LED counts must be 0..=512");
          mainboardRgb = next;
          return { saved: true, released: 0 } as T;
        }
        case "cooling.sessions.list":
          return { active: [...mockJobs.values()].filter((j) => ["authorizing", "executing", "stopping"].includes(j.state)).reverse(), max_active: 16,
            assignments: { resume_pending: false, assignments: [
              { control_id: "/lpc/it8689e/0/control/1", mode: "curve", temperature_id: "/lpc/it8689e/0/temperature/0", rpm_id: "/lpc/it8689e/0/fan/1", duty_percent: null,
                needs_attention: "Hardware identity or channel binding changed since the curve was assigned; check the channel and start it again" },
            ] } } as T;
        case "cooling.sessions.stop_all":
        case "cooling.sessions.stop_channel": {
          const active = [...mockJobs.values()].filter((j) => ["authorizing", "executing", "stopping"].includes(j.state)
            && (p.control_id === undefined || j.control_id === p.control_id));
          for (const j of active) setJob({ ...j, state: "stopped" });
          return { stopped: active.length } as T;
        }
        case "cooling.sessions.stop": {
          const stopped = mockJobs.get(p.job_id as number);
          if (stopped) setJob({ ...stopped, state: "stopped" });

          return true as T;
        }
        case "cooling.background.get":
          return background() as T;
        case "cooling.background.apply": {
          const channels = p.channels as BackgroundChannel[];
          for (const c of channels) coolingCheck(c.control_id, c.curve.minimum_percent);
          backgroundSaved = channels;
          fakeApply();
          return true as T;
        }
        case "cooling.support.get":
          return supportView() as T;
        case "cooling.support.settings":
          unrestricted = p.unrestricted_enabled as boolean;
          emit("cooling_support"); emit("cooling_background");
          return { background_reapplied: false } as T;
        case "cooling.channels.unlock": {
          const c = coolingChannels.find((x) => x.reading.id === p.control_id);
          if (!unrestricted) fail("Enable unrestricted control in the settings first");
          if (!c || (p.confirmation as string).trim() !== c.name) fail("Type the channel name exactly to confirm the unlock");
          if (c!.support?.software_mode) fail("This channel is already in software mode (another tool); it cannot be unlocked");
          evidence[c!.reading.id] = { ...evidence[c!.reading.id]!, unlock: "unlocked" };
          emit("cooling_support");
          return true as T;
        }
        case "cooling.channels.confirm_pump":
          fail("Pump confirmation applies only to channels bound as pump");
          return true as T;
        case "cooling.channels.revoke":
          evidence[p.control_id as string] = { ...evidence[p.control_id as string]!, unlock: "none" };
          emit("cooling_support");
          return { revoked: true, background_reapplied: false } as T;
        case "cooling.profiles.list":
          return slices().cooling_profiles as T;
        case "cooling.profiles.save":
          coolingProfiles.set(p.name as string, (p.channels as BackgroundChannel[] | undefined) ?? backgroundSaved);
          emit("cooling_profiles");
          return true as T;
        case "cooling.profiles.delete":
          coolingProfiles.delete(p.name as string);
          if (activeProfile === p.name) activeProfile = null;
          emit("cooling_profiles");
          return { deleted: true } as T;
        case "cooling.profiles.activate": {
          const channels = coolingProfiles.get(p.name as string) ?? fail("Unknown cooling profile");
          for (const c of channels) coolingCheck(c.control_id, c.curve.minimum_percent);
          backgroundSaved = channels;
          activeProfile = p.name as string;
          fakeApply(); emit("cooling_profiles");
          return { profile: p.name, started: [], restarting: [], background_applied: true } as T;
        }
        case "effects.get":
          return effect as T;
        case "diagnostics.get":
          return diagnostics() as T;
        case "effects.default":
          return (effectDefaults as Record<string, unknown>)[p.name as string] as T;
        case "modules.status":
          return setup.get() as T;
        case "modules.setup.start":
          setup.start(p.keys as string[]);
          return true as T;
        case "modules.setup.retry":
          setup.retry(p.keys as string[]);
          return true as T;
        case "modules.setup.complete":
          setup.complete();
          return true as T;
        case "modules.setup.dismiss":
          setup.dismiss(p.keys as string[]);
          return true as T;
        case "modules.light_test":
          setup.lightTest();
          return true as T;
        case "modules.setup.cancel":
          setup.cancel();
          return true as T;
        case "modules.catalog.refresh":
          return true as T;
        case "marketplace.status":
          return { packages: { source: "Browser preview — official examples", catalog: effectCatalog.effects, installed: [], job: null, error: null, failure: null, ratings: { configured: false, by_effect: {}, mine: {}, updated_at: null, loading: false, saving: null, error: null } }, active: effect, runtime_error: null, runtime_error_code: null } as T;
        case "cooling.curve_set.get":
          return curveSet as T;
        case "cooling.curve_set.save":
          curveSet = p.set as CurveSet;
          return true as T;
        case "cooling.curve_set.values": {
          // Sensor-driven nodes are not simulated in the browser: they fail safe.
          const outputs: Record<string, { percent: number; failsafe: boolean }> = {};
          const evaluate = (name: string, depth = 0): { percent: number; failsafe: boolean } => {
            const n = curveSet.nodes[name];
            if (outputs[name]) return outputs[name]!;
            const safe = { percent: 100, failsafe: true };
            const clamp = (v: number) => Math.min(100, Math.max(0, v));
            const r = !n || depth > 64 ? safe
              : n.type === "constant" ? { percent: n.percent, failsafe: false }
              : n.type === "follow" ? ((i) => (i.failsafe ? safe : { percent: clamp(i.percent * n.scale + n.offset_percent), failsafe: false }))(evaluate(n.source, depth + 1))
              : n.type === "mix" ? ((ins) => ins.some((i) => i.failsafe) ? safe : { percent: clamp(n.function === "maximum" ? Math.max(...ins.map((i) => i.percent)) : n.function === "minimum" ? Math.min(...ins.map((i) => i.percent)) : n.function === "sum" ? ins.reduce((a, i) => a + i.percent, 0) : ins.reduce((a, i) => a + i.percent, 0) / ins.length), failsafe: false })(n.sources.map((x) => evaluate(x, depth + 1)))
              : safe;
            return (outputs[name] = r);
          };
          Object.keys(curveSet.nodes).forEach((n) => evaluate(n));
          return { now_ms: Date.now(), outputs, simulated: true } as T;
        }
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
    wantSlices: async (requested) => { requested.forEach(emit); },
    startService: async () => {},
    exportDiagnostics: async () => {
      throw new Error("Not available with mock data");
    },
  };
}

