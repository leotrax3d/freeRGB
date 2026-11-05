import { macroProblem, type MacroDefinition, type MacroStep } from "@/lib/api/control";

/** Browser demo only: volatile pad events, never native input or persistence. */
export class MockMacroSupport {
  private session: string | null = null;
  private started = 0;
  private lastSeen = 0;
  private lastEvent = 0;
  private steps: MacroStep[] = [];
  private held = new Set<string>();
  private bindings: { device_id: string; usage: number; macro_name: string }[] = [];
  private enabled = false;
  call(method: string, p: Record<string, unknown>, names: string[]): unknown {
    const now = Date.now();
    if (this.session && (now - this.started >= 30_000 || now - this.lastSeen >= 3000)) this.clear();
    if (method === "macros.device_keys.get") return { bindings: this.bindings, enabled: this.enabled, physical_identity_pending: true, devices: [{ id: "mock:keyboard", name: "RGB Keyboard (virtual)", keys: { "63": 1, "64": 2, "65": 3, "66": 4, "67": 5 } }] };
    if (method === "macros.device_keys.save") {
      const bindings = p.bindings as typeof this.bindings;
      const seen = new Set<number>();
      if (!Array.isArray(bindings) || bindings.length > 64) throw new Error("Invalid bindings");
      for (const b of bindings) {
        if (b.device_id !== "mock:keyboard" || !Number.isInteger(b.usage) || b.usage < 63 || b.usage > 67 || !names.includes(b.macro_name) || seen.has(b.usage)) throw new Error("Unknown, duplicate or reserved device key");
        seen.add(b.usage);
      }
      this.bindings = bindings; this.enabled = false; return true;
    }
    if (method === "macros.device_keys.enable") { this.enabled = p.enabled === true; return true; }
    if (method === "macros.record.status") return { recording: !!this.session, steps: this.steps.length };
    if (method === "macros.record.start") {
      if (this.session || typeof p.session !== "string" || !p.session.length) throw new Error("Recording already active or missing session");
      this.session = p.session; this.started = this.lastSeen = this.lastEvent = now; return true;
    }
    if (!this.session || p.session !== this.session) throw new Error("Recording session is not active");
    if (method === "macros.record.stop") {
      const definition: MacroDefinition = { repeat: 1, steps: [...this.steps, ...[...this.held].map((key): MacroStep => ({ type: "key_up", key }))] };
      this.clear();
      if (p.discard) return null;
      const problem = macroProblem(definition); if (problem) throw new Error(problem);
      return definition;
    }
    const event = p.event as { type: string; key?: string; text?: string };
    if (event.type === "privacy_stop") { this.clear(); return true; }
    this.lastSeen = now;
    if (event.type === "heartbeat") return true;
    let step: MacroStep;
    if (event.type === "key_down" && event.key) {
      if (this.held.has(event.key)) return true;
      this.held.add(event.key); step = { type: "key_down", key: event.key };
    } else if (event.type === "key_up" && event.key && this.held.delete(event.key)) step = { type: "key_up", key: event.key };
    else if (event.type === "text" && event.text && !this.held.size) step = { type: "text", text: event.text };
    else { this.clear(); throw new Error("Invalid recording event"); }
    let delay = now - this.lastEvent;
    const previous = this.steps.at(-1);
    if (delay < 20 && previous?.type === "text" && step.type === "text" && previous.text.length + step.text.length <= 1024) previous.text += step.text;
    else {
      if (this.steps.length) while (delay > 0) { const ms = Math.min(delay, 10_000); this.steps.push({ type: "delay", ms }); delay -= ms; }
      this.steps.push(step);
    }
    this.lastEvent = now;
    if (this.steps.length + this.held.size > 128) { this.clear(); throw new Error("Recording step limit"); }
    return true;
  }
  private clear() { this.session = null; this.steps = []; this.held.clear(); }
}
