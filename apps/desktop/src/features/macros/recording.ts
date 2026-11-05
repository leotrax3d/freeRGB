import type { MacroDefinition } from "@/lib/api/control";

type Call = (method: string, params?: unknown) => Promise<unknown>;

/** Serializes input before stop; errors prevent a partially accepted draft. */
export class RecordingSession {
  private readonly id = `desktop-${crypto.randomUUID()}`;
  private queue: Promise<unknown> = Promise.resolve();
  private ended = false;
  constructor(private readonly call: Call) {}
  start() { return this.call("macros.record.start", { session: this.id }); }
  event(event: object) {
    if (this.ended) return Promise.reject(new Error("Recording ended"));
    this.queue = this.queue.then(() => this.call("macros.record.event", { session: this.id, event }));
    return this.queue;
  }
  async stop(discard: boolean): Promise<MacroDefinition | null> {
    this.ended = true;
    try { await this.queue; }
    catch (e) {
      await this.call("macros.record.stop", { session: this.id, discard: true }).catch(() => {});
      throw e;
    }
    return await this.call("macros.record.stop", { session: this.id, discard }) as MacroDefinition | null;
  }
}

export function recordedKey(key: string): string | null {
  const name = ({ Control: "ctrl", Alt: "alt", Shift: "shift", Meta: "meta", " ": "space", ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" } as Record<string, string>)[key] ?? key.toLowerCase();
  return /^([a-z0-9]|ctrl|alt|shift|meta|enter|escape|tab|space|backspace|up|down|left|right)$/.test(name) ? name : null;
}
