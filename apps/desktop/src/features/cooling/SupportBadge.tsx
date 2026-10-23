import type { SupportTier } from "@/lib/api/types";
import { StatusDot } from "@/components/status";
import { tierDescription, tierLabel, tierTone } from "./support";

/** Tier badge with an explanatory tooltip; colour is never the only signal. */
export function SupportBadge({ tier }: { tier: SupportTier | null | undefined }) {
  return (
    <span title={tierDescription(tier)}>
      <StatusDot tone={tierTone(tier)} label={`Support: ${tierLabel(tier)}`} />
    </span>
  );
}
