import type { ChannelBinding, CoolingChannel, MonitoringSensor } from "@/lib/api/types";

/** What the wizard knows about one motherboard fan header. */
export interface FanHeader {
  control: CoolingChannel;
  hardware: string;
  index: string;
  rpm: CoolingChannel | null;
  temperature: MonitoringSensor | null;
  /** Why it cannot be identified, if it cannot. */
  blocked: string | null;
}

/** `/lpc/chip/0/control/2` -> ["/lpc/chip/0", "2"]. */
export function splitId(id: string): [string, string] | null {
  const match = /^(.*)\/(control|fan|temperature)\/(\d+)$/.exec(id);
  return match ? [match[1]!, match[3]!] : null;
}

/** Pairs each control with the tachometer of the same index and a board temperature. */
export function fanHeaders(channels: CoolingChannel[], sensors: MonitoringSensor[]): FanHeader[] {
  return channels
    .filter((c) => c.kind === "control")
    .flatMap((control) => {
      const parts = splitId(control.reading.id);
      if (!parts) return [];
      const [hardware, index] = parts;
      const rpm = channels.find((c) => c.kind === "fan" && c.reading.id === `${hardware}/fan/${index}`) ?? null;
      const temperature = sensors.find((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C" && s.sensor.id.startsWith(`${hardware}/temperature/`)) ?? null;
      let blocked: string | null = null;
      if (!control.control_available) blocked = "No usable fan control on this header.";
      else if (control.support?.software_mode) blocked = "Another program already controls this header.";
      else if (control.support?.held_full) blocked = "Held at 100 % after a crash; restart Windows first.";
      else if (!rpm) blocked = "No matching RPM sensor, so freeRGB cannot watch the fan during the test.";
      else if (!temperature) blocked = "No temperature sensor on this chip for the safety watchdog.";
      return [{ control, hardware, index, rpm, temperature, blocked }];
    });
}

/** Insert or replace the binding of one control; header names stay unique per board. */
export function upsertBinding(bindings: ChannelBinding[], next: ChannelBinding): ChannelBinding[] {
  const others = bindings.filter((b) => b.control_id !== next.control_id);
  if (others.some((b) => b.device_id === next.device_id && b.header.toLowerCase() === next.header.toLowerCase()))
    throw new Error(`The header name "${next.header}" is already used on this board.`);
  if (next.tachometer_id && others.some((b) => b.tachometer_id === next.tachometer_id))
    throw new Error("This RPM sensor is already assigned to another header.");
  return [...others, next];
}
