import type { ChannelCoverage, CoolingChannel, CoolingSupport, SupportTier } from "@/lib/api/types";
import type { Tone } from "@/components/status";

/** Chip and coverage status (docs/FAN_COVERAGE.md) for a channel, or null. */
export function coverageLabel(coverage: ChannelCoverage | null | undefined): string | null {
  if (!coverage) return null;
  const status = coverage.status === "hardware-verified" ? "hardware-verified" : coverage.status === "simulated" ? "simulated only" : "not covered";
  return `${coverage.chip} · ${status}`;
}

/** Short, user-facing names for the support tiers (DECISIONS.md #14). */
export function tierLabel(tier: SupportTier | null | undefined): string {
  switch (tier) {
    case "verified": return "Verified";
    case "lhm_supported": return "LHM-supported";
    case "unrestricted": return "Unrestricted";
    case "read_only": return "Read-only";
    default: return "Unknown";
  }
}

export function tierDescription(tier: SupportTier | null | undefined): string {
  switch (tier) {
    case "verified": return "Every write is checked by reading the chip's registers back.";
    case "lhm_supported": return "LibreHardwareMonitor can drive this header; freeRGB checks the fan's RPM response instead of the registers. Reduced speeds need a passed channel test.";
    case "unrestricted": return "Unlocked by you for untested hardware: down to 0 % (fan stop), deviations are only warnings.";
    case "read_only": return "No usable fan control on this channel.";
    default: return "The protected service did not report support for this channel.";
  }
}

export function tierTone(tier: SupportTier | null | undefined): Tone {
  return tier === "verified" ? "ok" : tier === "lhm_supported" || tier === "unrestricted" ? "warn" : "idle";
}

/** Why the daemon will refuse to control this channel, if it will. */
export function controlRefusal(support: CoolingSupport | null | undefined): string | null {
  if (support?.software_mode) return "Another program already controls this header (software mode); freeRGB will not take it over.";
  if (support?.held_full) return "Held at 100 % after a worker crash. Restart Windows to return it to firmware control.";
  return null;
}

/** Capability of a control channel as reported by the protected inventory. */
export function channelTier(channel: CoolingChannel | undefined): SupportTier | null {
  if (!channel || channel.kind !== "control") return null;
  if (!channel.control_available) return "read_only";
  return channel.support?.tier ?? null;
}
