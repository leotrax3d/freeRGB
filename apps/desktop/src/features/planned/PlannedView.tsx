import { Construction } from "lucide-react";
import { useNavigation } from "@/app/navigation";
import { VIEWS } from "@/app/views";
import { EmptyState } from "@/components/status";

/** Placeholder for views not migrated yet; points to the Classic GUI meanwhile. */
export default function PlannedView() {
  const view = useNavigation((s) => s.view);
  const def = VIEWS.find((v) => v.id === view);
  return (
    <EmptyState icon={<Construction />} title={`${def?.label ?? "This view"} is coming in ${def?.planned ?? "a later step"}`}>
      Until then, use freeRGB Classic for this area. Both apps control the same background service, and every
      setting is shared.
    </EmptyState>
  );
}
