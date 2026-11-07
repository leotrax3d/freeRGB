import { t } from "./messages";
import type { ChannelCoverage, SupportTier } from "@/lib/api/types";
import { StatusDot } from "@/components/status";
import { coverageLabel, tierDescription, tierLabel, tierTone } from "./support";

/** Tier badge with an explanatory tooltip; colour is never the only signal. */
export function SupportBadge({ tier, coverage }: { tier: SupportTier | null | undefined; coverage?: ChannelCoverage | null }) {
  const chip = coverageLabel(coverage);
  return (
    <span title={tierDescription(tier)} className="inline-flex items-center gap-2">
      <StatusDot tone={tierTone(tier)} label={t("cooling.supportBadge.supportValue", { p0: tierLabel(tier) })} />
      {chip && <span title={t("cooling.supportBadge.coveragePerFanControllerChipFanCoverage")}>{chip}</span>}
    </span>
  );
}
