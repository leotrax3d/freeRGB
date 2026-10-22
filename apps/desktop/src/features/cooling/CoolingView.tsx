import { ShieldAlert } from "lucide-react";
import { PageHeader } from "@/components/page";
import { useDaemon, useSlices } from "@/lib/store/daemon";
import { CurvesSection } from "./CurvesSection";
import { SensorsSection } from "./SensorsSection";
import { LiveSession } from "./LiveSession";
import { ChannelBindings } from "./ChannelBindings";

export default function CoolingView() {
  useSlices("curves", "cooling_status", "sensor_graph", "sensor_sources", "sensor_values");
  const status = useDaemon((s) => s.cooling_status);
  return (
    <>
      <PageHeader title="Cooling" description="Fan curves and temperature sources." />
      {status && !status.hardware_control && (
        <div role="note" className="flex items-start gap-3 rounded-card border border-border bg-surface-sunken px-4 py-3 text-small text-text-muted">
          <ShieldAlert className="mt-0.5 size-4 shrink-0 text-warning" />
          <span>
            <span className="font-medium text-text">Curve editor preview does not change fan speeds. Live control has a separate start action below.</span>{" "}
            {status.reason}
          </span>
        </div>
      )}
      <div className="@container flex flex-col gap-8">
        <CurvesSection />
        <LiveSession />
        <ChannelBindings />
        <SensorsSection />
      </div>
    </>
  );
}
