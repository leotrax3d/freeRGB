import { t } from "./messages";
import { useState } from "react";
import { ChevronRight, Wand2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CoolingSetup } from "./CoolingSetup";
import { PageHeader } from "@/components/page";
import { useSlices } from "@/lib/store/daemon";
import { CurvesSection } from "./CurvesSection";
import { SensorsSection } from "./SensorsSection";
import { CompositeCurvesSection } from "./CompositeCurvesSection";
import { LiveSession } from "./LiveSession";
import { ChannelBindings } from "./ChannelBindings";
import { UnrestrictedSection } from "./UnrestrictedSection";
import { CoolingActions, FanOverview } from "./FanOverview";
import { HwmonSection } from "./HwmonSection";
import { GpuFansSection, SourcesSection } from "./SourcesSection";
import { PluginChannelsSection } from "./PluginsSection";

/** Fans first: what each header does now and one place to change it. */
export default function CoolingView() {
  useSlices("curves", "cooling_status", "sensor_graph", "sensor_sources", "sensor_values");
  const [setup, setSetup] = useState(false);
  return (
    <>
      <PageHeader title={t("cooling.coolingView.cooling")} description={t("cooling.coolingView.yourFansWhatControlsThemAndThe")} actions={<div className="flex flex-col items-end gap-2"><Button size="sm" variant={setup ? "secondary" : "primary"} aria-expanded={setup} onClick={() => setSetup((s) => !s)}><Wand2 />{t("cooling.coolingView.guidedFanSetup")}</Button><CoolingActions /></div>} />
      <div className="@container flex flex-col gap-8">
        {setup && <CoolingSetup onClose={() => setSetup(false)} />}
        <FanOverview />
        <HwmonSection />
        <GpuFansSection />
        <PluginChannelsSection />
        <LiveSession />
        <CurvesSection />
        <details className="group flex flex-col gap-6 rounded-card border border-border px-4 py-3">
          <summary className="flex cursor-pointer list-none items-center gap-2 text-small font-medium text-text-subtle">
            <ChevronRight className="size-4 transition-transform duration-(--duration-base) group-open:rotate-90" aria-hidden />
            {t("cooling.coolingView.advancedUnrestrictedControlHeaderLabelsTemperatureSources")} </summary>
          <div className="mt-4 flex flex-col gap-8">
            <UnrestrictedSection />
            <ChannelBindings />
            <SourcesSection />
            <SensorsSection />
            <CompositeCurvesSection />
          </div>
        </details>
      </div>
    </>
  );
}
