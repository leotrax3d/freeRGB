import { useEffect, useRef, useState } from "react";
import { AlertTriangle, Check, CircleSlash, Cpu, Fan, ShieldAlert, Sparkles } from "lucide-react";
import { BrandLoader, BrandRing, BrandWordmark } from "@/components/brand";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useNavigation } from "@/app/navigation";
import { SetupWizard } from "@/features/setup/SetupWizard";
import { summary } from "@/features/setup/model";
import { FanStep, type FanOutcome } from "./FanStep";
import { InlineError } from "@/components/status";
import { t, type OnboardingKey } from "./messages";
import { SAFETY_CONFIRMATIONS, SAFETY_RISKS, SAFETY_TERMS, SAFETY_TEXT_VERSION, SAFETY_TITLE } from "./safety";

type StepId = "welcome" | "safety" | "devices" | "fans" | "summary";
const STEPS: { id: StepId; label: OnboardingKey }[] = [
  { id: "welcome", label: "onboarding.step.welcome" },
  { id: "safety", label: "onboarding.step.safety" },
  { id: "devices", label: "onboarding.step.devices" },
  { id: "fans", label: "onboarding.step.fans" },
  { id: "summary", label: "onboarding.step.summary" },
];

function Progress({ step }: { step: StepId }) {
  const index = STEPS.findIndex((s) => s.id === step);
  return (
    <ol className="flex items-center gap-2" aria-label={t("onboarding.progress")}>
      {STEPS.map((s, i) => (
        <li key={s.id} className="flex items-center gap-2" aria-current={i === index ? "step" : undefined}>
          <span className={`flex size-6 items-center justify-center rounded-full text-caption transition-colors duration-(--duration-base) ${i < index ? "bg-accent text-black" : i === index ? "bg-text text-black" : "bg-surface-raised text-text-subtle"}`}>
            {i < index ? <Check className="size-3.5" aria-hidden /> : i + 1}
          </span>
          <span className={`hidden text-small sm:inline ${i === index ? "text-text" : "text-text-subtle"}`}>{t(s.label)}</span>
          {i < STEPS.length - 1 && <span className={`h-px w-6 ${i < index ? "bg-accent" : "bg-border"}`} aria-hidden />}
        </li>
      ))}
    </ol>
  );
}

function Safety({ onAccepted }: { onAccepted: () => void }) {
  const [checked, setChecked] = useState<boolean[]>(SAFETY_CONFIRMATIONS.map(() => false));
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const accept = async () => {
    setPending(true);
    setError(null);
    try {
      await rpc("safety.acknowledge", { version: SAFETY_TEXT_VERSION, accepted: true });
      onAccepted();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };
  return (
    <div role="alertdialog" aria-labelledby="safety-title" aria-describedby="safety-risks"
      className="flex flex-col gap-5 rounded-card border-2 border-danger bg-[#5c0b0b] p-6 text-text">
      <div className="flex items-start gap-3">
        <ShieldAlert className="mt-0.5 size-8 shrink-0 text-danger-text" aria-hidden />
        <div className="flex flex-col gap-1">
          <h2 id="safety-title" className="text-title font-semibold text-danger-text">{SAFETY_TITLE}</h2>
          <p className="text-small text-text">{t("onboarding.safety.intro")}</p>
          <p className="text-caption text-text-muted">{t("onboarding.safety.langNote")}</p>
        </div>
      </div>
      <ul id="safety-risks" className="grid gap-3 sm:grid-cols-2">
        {SAFETY_RISKS.map((r) => (
          <li key={r.title} className="flex gap-2 rounded-control border border-danger/40 bg-black/25 p-3">
            <AlertTriangle className="mt-0.5 size-4 shrink-0 text-danger-text" aria-hidden />
            <span className="flex flex-col gap-0.5"><strong className="text-small">{r.title}</strong><span className="text-small text-text-muted">{r.text}</span></span>
          </li>
        ))}
      </ul>
      <div className="flex flex-col gap-2 text-small">
        {SAFETY_TERMS.map((term) => <p key={term}>{term}</p>)}
      </div>
      <div className="flex flex-col gap-2 rounded-control bg-black/25 p-3">
        {SAFETY_CONFIRMATIONS.map((text, i) => (
          <Checkbox key={text} checked={checked[i]!} onChange={(on) => setChecked((c) => c.map((v, j) => (j === i ? on : v)))}>{text}</Checkbox>
        ))}
      </div>
      {error && <InlineError>{error}</InlineError>}
      {confirming ? (
        // Second, explicit question before fan control is unlocked.
        <div role="alertdialog" aria-labelledby="safety-confirm" className="flex flex-col gap-3 rounded-control border border-danger bg-black/40 p-4 animate-[fade-in_var(--duration-base)_var(--ease-out)]">
          <strong id="safety-confirm" className="text-body">{t("onboarding.safety.sure")}</strong>
          <p className="text-small text-text-muted">{t("onboarding.safety.sureText")}</p>
          <div className="flex flex-wrap justify-end gap-2">
            <Button variant="ghost" disabled={pending} onClick={() => setConfirming(false)}>{t("onboarding.safety.back")}</Button>
            <Button variant="danger" disabled={pending} onClick={() => void accept()}>{t("onboarding.safety.yes")}</Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <span className="text-caption text-text-muted">{t("onboarding.safety.without")}</span>
          <Button variant="danger" disabled={!checked.every(Boolean) || pending} onClick={() => setConfirming(true)}>{t("onboarding.safety.accept")}</Button>
        </div>
      )}
    </div>
  );
}

/** First-run wizard (docs/SETUP_WIZARD.md); reopen from Settings. */
export function OnboardingWizard() {
  const onboarding = useDaemon((s) => s.onboarding);
  const modules = useDaemon((s) => s.modules_setup);
  const go = useNavigation((s) => s.go);
  const [step, setStep] = useState<StepId>("welcome");
  const [skippedSafety, setSkippedSafety] = useState(false);
  const [fans, setFans] = useState<Record<string, FanOutcome>>({});
  const [closing, setClosing] = useState(false);
  const [finishError, setFinishError] = useState<string | null>(null);
  const heading = useRef<HTMLDivElement>(null);
  // The whole screen turns red while the safety notice is open.
  const warning = step === "safety" && !onboarding?.safety_acknowledged;
  useEffect(() => heading.current?.focus({ preventScroll: true }), [step]);
  if (!onboarding || onboarding.completed) return null;

  const finish = async () => {
    setClosing(true);
    setFinishError(null);
    try {
      await rpc("modules.setup.complete");
      await rpc("onboarding.complete");
      go("lighting");
    } catch (error) {
      setFinishError(error instanceof Error ? error.message : String(error));
    } finally {
      setClosing(false);
    }
  };
  const counts = (outcome: FanOutcome) => Object.values(fans).filter((o) => o === outcome).length;
  const devices = summary(modules?.cards.filter((c) => c.phase !== "unsupported") ?? []);
  const unsupported = modules?.cards.filter((c) => c.phase === "unsupported").length ?? 0;

  return (
    <div role="dialog" aria-modal="true" aria-label={t("onboarding.dialog")} className={`fixed inset-0 z-50 flex justify-center overflow-y-auto transition-colors duration-(--duration-base) animate-[fade-in_var(--duration-base)_var(--ease-out)] ${warning ? "bg-[#3a0606]" : "bg-bg"}`}>
      <div className="flex w-full max-w-4xl flex-col gap-8 px-8 py-10">
        <header className="flex flex-wrap items-center justify-between gap-4">
          <span className="flex items-center gap-3"><BrandWordmark height={28} /><span className="text-small text-text-subtle">{t("onboarding.setup")}</span></span>
          <Progress step={step} />
        </header>
        <div key={step} ref={heading} tabIndex={-1} className="flex flex-col gap-6 outline-none animate-[view-in_var(--duration-base)_var(--ease-out)]">
          {step === "welcome" && (
            <div className="flex flex-col gap-4">
              <BrandLoader size={112} label="freeRGB" className="animate-[pop-in_var(--duration-base)_var(--ease-out)_both]" />
              <h1 className="text-page font-semibold tracking-tight">{t("onboarding.welcome.title")}</h1>
              <p className="text-body text-text-muted">{t("onboarding.welcome.text")}</p>
              <ul className="grid gap-3 sm:grid-cols-3">
                {[
                  { icon: <ShieldAlert />, title: t("onboarding.welcome.safetyTitle"), text: t("onboarding.welcome.safetyText") },
                  { icon: <Sparkles />, title: t("onboarding.welcome.devicesTitle"), text: t("onboarding.welcome.devicesText") },
                  { icon: <Fan />, title: t("onboarding.welcome.fansTitle"), text: t("onboarding.welcome.fansText") },
                ].map((item, i) => (
                  <li key={item.title} style={{ animationDelay: `${120 + i * 90}ms` }}
                    className="flex flex-col gap-2 rounded-card border border-border bg-surface p-4 animate-[pop-in_var(--duration-base)_var(--ease-out)_both]">
                    <span className="text-accent [&_svg]:size-5" aria-hidden>{item.icon}</span>
                    <strong className="text-small">{item.title}</strong>
                    <span className="text-small text-text-muted">{item.text}</span>
                  </li>
                ))}
              </ul>
              <div className="flex gap-3"><Button variant="primary" onClick={() => setStep("safety")}>{t("onboarding.welcome.start")}</Button></div>
            </div>
          )}
          {step === "safety" && (
            onboarding.safety_acknowledged ? (
              <div className="flex flex-col gap-4">
                <h1 className="text-page font-semibold tracking-tight">{t("onboarding.accepted.title")}</h1>
                <p className="text-body text-text-muted">{t("onboarding.accepted.text")}</p>
                <div><Button variant="primary" onClick={() => setStep("devices")}>{t("onboarding.continue")}</Button></div>
              </div>
            ) : (
              <>
                <Safety onAccepted={() => setStep("devices")} />
                <div><Button variant="ghost" onClick={() => { setSkippedSafety(true); setStep("devices"); }}>{t("onboarding.skipSafety")}</Button></div>
              </>
            )
          )}
          {step === "devices" && (
            <div className="flex flex-col gap-4">
              <SetupWizard embedded onDone={() => setStep(onboarding.safety_acknowledged ? "fans" : "summary")} />
            </div>
          )}
          {step === "fans" && (
            <div className="flex flex-col gap-4">
              <h1 className="text-page font-semibold tracking-tight">{t("onboarding.fans.title")}</h1>
              <FanStep onDone={(outcomes) => { setFans(outcomes); setStep("summary"); }} />
            </div>
          )}
          {step === "summary" && (
            <div className="flex flex-col gap-5">
              <BrandRing size={56} className="animate-[pop-in_var(--duration-base)_var(--ease-out)_both]" />
              <h1 className="text-page font-semibold tracking-tight">{t("onboarding.summary.title")}</h1>
              <ul className="grid gap-3 sm:grid-cols-2">
                <Summary icon={<Sparkles />} title={t("onboarding.summary.lighting")} text={`${t("onboarding.summary.ready", { ready: devices.ready })}${devices.failed ? t("onboarding.summary.failed", { failed: devices.failed }) : ""}${unsupported ? t("onboarding.summary.unsupported", { n: unsupported }) : ""}`} />
                <Summary icon={<Fan />} title={t("onboarding.summary.fans")} text={onboarding.safety_acknowledged ? t("onboarding.summary.fansText", { fan: counts("fan"), pump: counts("pump"), none: counts("none") + counts("skipped"), blocked: counts("blocked") }) : t("onboarding.summary.fansLocked")} />
                <Summary icon={<Cpu />} title={t("onboarding.summary.service")} text={t("onboarding.summary.serviceText")} />
                {skippedSafety && <Summary icon={<CircleSlash />} title={t("onboarding.summary.safety")} text={t("onboarding.summary.safetyText")} />}
              </ul>
              <p className="text-small text-text-muted">{t("onboarding.summary.note")}</p>
              {finishError && <InlineError>{finishError}</InlineError>}
              <div><Button variant="primary" disabled={closing} onClick={() => void finish()}>{closing ? t("onboarding.summary.saving") : t("onboarding.summary.open")}</Button></div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function Summary({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) {
  return (
    <li className="flex gap-3 rounded-card border border-border bg-surface p-4 animate-[pop-in_var(--duration-base)_var(--ease-out)_both]">
      <span className="text-accent [&_svg]:size-5" aria-hidden>{icon}</span>
      <span className="flex flex-col gap-0.5"><strong className="text-small">{title}</strong><span className="text-small text-text-muted">{text}</span></span>
    </li>
  );
}
