import { t } from "./messages";
import type { ChannelCoverage, CoolingChannel, CoolingSupport, SupportTier } from "@/lib/api/types";
import type { Tone } from "@/components/status";

/** Chip and coverage status (docs/FAN_COVERAGE.md) for a channel, or null. */
export function coverageLabel(coverage: ChannelCoverage | null | undefined): string | null {
  if (!coverage) return null;
  const status = coverage.status === "hardware-verified" ? t("cooling.coverage.verified") : coverage.status === "simulated" ? t("cooling.support.simulatedOnly") : t("cooling.support.notCovered");
  return `${coverage.chip} · ${status}`;
}

/** Short, user-facing names for the support tiers (DECISIONS.md #14). */
export function tierLabel(tier: SupportTier | null | undefined): string {
  switch (tier) {
    case "verified": return t("cooling.support.verified");
    case "lhm_supported": return t("cooling.support.lhmSupported");
    case "unrestricted": return t("cooling.support.unrestricted");
    case "read_only": return t("cooling.support.readOnly");
    default: return t("cooling.support.unknown");
  }
}

export function tierDescription(tier: SupportTier | null | undefined): string {
  switch (tier) {
    case "verified": return t("cooling.support.everyWriteIsCheckedByReadingThe");
    case "lhm_supported": return t("cooling.support.librehardwaremonitorCanDriveThisHeaderFreergbChecks");
    case "unrestricted": return t("cooling.support.unlockedByYouForUntestedHardwareDown");
    case "read_only": return t("cooling.support.noUsableFanControlOnThisChannel");
    default: return t("cooling.support.theProtectedServiceDidNotReportSupport");
  }
}

export function tierTone(tier: SupportTier | null | undefined): Tone {
  return tier === "verified" ? "ok" : tier === "lhm_supported" || tier === "unrestricted" ? "warn" : "idle";
}

/** Why the daemon will refuse to control this channel, if it will. */
export function controlRefusal(support: CoolingSupport | null | undefined): string | null {
  if (support?.software_mode) return t("cooling.support.anotherProgramAlreadyControlsThisHeaderSoftware");
  if (support?.held_full) return t("cooling.support.heldAt100AfterAWorkerCrash");
  return null;
}

/** Capability of a control channel as reported by the protected inventory. */
export function channelTier(channel: CoolingChannel | undefined): SupportTier | null {
  if (!channel || channel.kind !== "control") return null;
  if (!channel.control_available) return "read_only";
  return channel.support?.tier ?? null;
}
