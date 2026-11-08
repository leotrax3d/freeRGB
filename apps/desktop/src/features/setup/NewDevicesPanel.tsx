import { History, PackageCheck } from "lucide-react";
import { useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { bytes, isWorking, newDevices, selectable, selectionSize, supported } from "./model";
import { InlineError } from "@/components/status";
import { t } from "./messages";
import { SetupCardView } from "./SetupCard";
import { useSetup } from "./useSetup";

/**
 * Small panel in the device overview for devices found after the first
 * setup, plus failed, not connected and unsupported ones. Same flow as the
 * first-start setup; existing devices keep running meanwhile.
 */
export function NewDevicesPanel() {
  const setup = useSetup();
  const [deselected, setDeselected] = useState<Set<string>>(new Set());
  const status = setup.status;
  if (!status || status.first_run) return null;
  const cards = status.cards;
  const fresh = newDevices(cards);
  const attention = cards.filter(
    (c) => supported(c) && c.configured && (c.phase === "failed" || c.phase === "not_connected" || isWorking(c)),
  );
  const working = fresh.filter(isWorking);
  const offer = fresh.filter(selectable);
  const chosen = offer.filter((c) => !deselected.has(c.key)).map((c) => c.key);
  const size = selectionSize(cards, chosen);
  if (fresh.length === 0 && attention.length === 0) return null;

  return (
    <div className="flex flex-col gap-6">
      {(fresh.length > 0 || attention.length > 0) && (
        <section
          aria-label={t("setup.panel.aria")}
          className="flex flex-col gap-3 rounded-card border border-border-strong bg-surface-raised/40 p-4 animate-[pop-in_var(--duration-slow)_var(--ease-out)]"
        >
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-col">
              <h2 className="text-title font-semibold">{fresh.length > 0 ? t("setup.panel.found") : t("setup.panel.attention")}</h2>
              <p className="text-small text-text-muted">{t("setup.panel.running")}</p>
            </div>
            {offer.length > 0 && working.length === 0 && (
              <div className="flex items-center gap-2">
                {size.bytes > 0 && <span className="text-small text-text-subtle tabular">{bytes(size.bytes)}</span>}
                <Button variant="ghost" size="sm" onClick={() => void setup.dismiss(offer.map((c) => c.key))}>
                  {t("setup.panel.later")}
                </Button>
                <Button variant="primary" size="sm" disabled={chosen.length === 0} onClick={() => void setup.start(chosen)}>
                  {t("setup.panel.setUp")}
                </Button>
              </div>
            )}
            {status.job.running && (
              <Button variant="ghost" size="sm" onClick={() => void setup.cancel()}>
                {t("setup.panel.cancel")}
              </Button>
            )}
          </div>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
            {[...fresh, ...attention].map((c, i) => (
              <SetupCardView
                key={c.key}
                card={c}
                index={i}
                compact
                selected={selectable(c) && !c.configured ? !deselected.has(c.key) : undefined}
                onSelect={(on) => {
                  const next = new Set(deselected);
                  if (on) next.delete(c.key);
                  else next.add(c.key);
                  setDeselected(next);
                }}
                onRetry={c.phase === "failed" ? () => void setup.retry(c.key) : undefined}
                onForget={() => void setup.forget([c.key])}
              />
            ))}
          </div>
          {setup.error && <InlineError>{setup.error}</InlineError>}
        </section>
      )}

    </div>
  );
}

/** Below the device grid: unsupported devices and installed modules. */
export function SetupExtras() {
  const setup = useSetup();
  const status = setup.status;
  if (!status) return null;
  const unsupported = status.cards.filter((c) => c.phase === "unsupported");
  if (unsupported.length === 0 && status.installed.length === 0) return null;
  return (
    <div className="flex flex-col gap-6">
      {unsupported.length > 0 && (
        <Section title={t("setup.extras.unsupported")}>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
            {unsupported.map((c, i) => (
              <SetupCardView key={c.key} card={c} index={i} compact />
            ))}
          </div>
        </Section>
      )}

      {status.installed.length > 0 && (
        <Section title={t("setup.extras.modules")}>
          <ul className="flex flex-col divide-y divide-border rounded-card border border-border">
            {status.installed.map((m) => (
              <li key={m.id} className="flex flex-wrap items-center gap-3 px-4 py-3 text-small">
                <PackageCheck className="size-4 text-text-subtle" aria-hidden />
                <span className="font-medium text-text">{m.id}</span>
                <span className="text-text-subtle tabular">v{m.version}</span>
                <span className="text-text-subtle">{m.license}</span>
                <span className="flex-1" />
                {(m.previous || (m.last_good && m.last_good !== m.version)) && (
                  <Button size="sm" variant="ghost" disabled={setup.pending === `rollback:${m.id}`} onClick={() => void setup.rollback(m.id)}>
                    <History aria-hidden /> {t("setup.extras.previous")}
                  </Button>
                )}
              </li>
            ))}
          </ul>
          <p className="text-caption text-text-subtle">
            {t("setup.extras.note", { source: status.catalog.source, unavailable: status.catalog.state === "error" ? t("setup.extras.unavailable") : "" })}
          </p>
        </Section>
      )}
    </div>
  );
}
