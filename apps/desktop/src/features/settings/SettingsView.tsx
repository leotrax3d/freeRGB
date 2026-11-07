import { Contact } from "@/components/Contact";
import { DeveloperContact } from "./DeveloperContact";
import { DeveloperInbox } from "./DeveloperInbox";
import { Compass, Download, Puzzle } from "lucide-react";
import { useTour } from "@/app/Tour";
import { PageHeader, Section } from "@/components/page";
import { InlineError, StatusDot } from "@/components/status";
import { Button } from "@/components/ui/button";
import { deviceStatus } from "@/lib/api/types";
import { num } from "@/lib/format";
import { LOCALES, useLocale, type LocalePreference } from "@/lib/i18n";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { KeyboardReactions } from "@/features/devices/KeyboardReactions";
import { transport } from "@/lib/api/transport";
import { useState } from "react";
import type { UpdateInfo } from "@/lib/api/types";
import { t } from "./messages";

function Plugins() {
  const plugins = useDaemon((s) => s.plugins);
  const { pending, error, run } = useAction();
  return (
    <Section title={t("settings.plugins")} aside={error && <InlineError>{error}</InlineError>}>
      {!plugins?.length ? (
        <p className="text-small text-text-subtle">{t("settings.plugins.none")}</p>
      ) : (
        <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
          {plugins.map((p) => {
            const state = p.status ? deviceStatus(p.status) : null;
            return (
              <li key={p.id} className="flex items-center gap-4 px-4 py-3">
                <Puzzle className="size-4 text-text-subtle" aria-hidden />
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-body">{p.name}</span>
                  <span className="text-caption text-text-subtle">
                    {p.id} · {p.access === "hardware" ? t("settings.plugins.hardware") : t("settings.plugins.virtual")} ·{" "}
                    {t("settings.plugins.frames", { n: num(p.frames) })}
                  </span>
                  {p.last_error && <InlineError>{p.last_error}</InlineError>}
                </div>
                <StatusDot
                  tone={state === "connected" ? "ok" : state === "faulted" ? "error" : "idle"}
                  label={
                    p.enabled
                      ? state === "connected"
                        ? t("settings.plugins.running")
                        : state === "faulted"
                          ? t("settings.plugins.error")
                          : t("settings.plugins.starting")
                      : t("settings.plugins.stopped")
                  }
                />
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={!p.allowed || pending !== null}
                  title={p.allowed ? undefined : t("settings.plugins.notAllowed")}
                  onClick={() => run(p.id, () => rpc(p.enabled ? "plugins.stop" : "plugins.start", { id: p.id }))}
                >
                  {p.enabled ? t("settings.plugins.stop") : t("settings.plugins.start")}
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </Section>
  );
}

/** Display language: automatic (system) or an explicit choice; applies immediately. */
function LanguageSettings() {
  const preference = useLocale((s) => s.preference);
  const setPreference = useLocale((s) => s.setPreference);
  return (
    <Section title={t("settings.language")}>
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
        <label htmlFor="language-select" className="flex flex-col text-small">
          <span>{t("settings.language.label")}</span>
          <span className="text-caption text-text-subtle">{t("settings.language.hint")}</span>
        </label>
        <select
          id="language-select"
          value={preference}
          onChange={(e) => setPreference(e.target.value as LocalePreference)}
          className="min-h-9 max-w-full rounded-control border border-border bg-surface-sunken px-3 py-2 text-small"
        >
          <option value="auto">{t("settings.language.auto")}</option>
          {LOCALES.map((l) => (
            <option key={l.id} value={l.id} lang={l.id} dir="auto">
              {l.label}{l.machineTranslated ? ` ? ${t("settings.language.machineTranslated")}` : ""}
            </option>
          ))}
        </select>
      </div>
    </Section>
  );
}

export default function SettingsView() {
  useSlices("plugins");
  const connection = useDaemon((s) => s.connection);
  const { pending, error, run } = useAction();
  return (
    <>
      <PageHeader title={t("settings.title")} />
      <LanguageSettings />
      <Plugins />
      <KeyboardReactions />
      <DeveloperContact />
      <DeveloperInbox />
      <Section title={t("settings.classic")}>
        <p className="text-small text-text-muted">{t("settings.classic.text")}</p>
        <Button
          variant="secondary"
          disabled={pending !== null || useDaemon.getState().mode !== "tauri"}
          onClick={() =>
            run("classic", async () => {
              const tr = await transport();
              if (!tr.openClassic) throw new Error(t("settings.classic.unavailable"));
              await tr.openClassic();
            })
          }
        >
          {t("settings.classic.open")}
        </Button>
        {error && <InlineError>{error}</InlineError>}
      </Section>
      <Updates />
      <SetupSettings />
      <Section title={t("settings.help")}>
        <div className="flex items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
          <span className="text-small text-text-muted">{t("settings.help.text")}</span>
          <Button variant="secondary" size="sm" onClick={() => useTour.getState().start()}>
            <Compass aria-hidden /> {t("settings.help.tour")}
          </Button>
        </div>
      </Section>
      <Section title={t("settings.contact")}><Contact /></Section>
      <Section title={t("settings.about")}>
        <dl className="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1.5 text-small">
          <dt className="text-text-subtle">{t("settings.about.app")}</dt>
          <dd>freeRGB desktop {__APP_VERSION__}</dd>
          <dt className="text-text-subtle">{t("settings.about.service")}</dt>
          <dd>{connection?.daemon_version ? `freergbd ${connection.daemon_version} · API ${connection.api_version}` : t("settings.about.notConnected")}</dd>
          <dt className="text-text-subtle">{t("settings.about.classic")}</dt>
          <dd className="text-text-muted">{t("settings.about.classicText")}</dd>
        </dl>
      </Section>
    </>
  );
}

/** Re-run the first-run wizard; show or withdraw the fan-control safety acknowledgement. */
function SetupSettings() {
  const onboarding = useDaemon((s) => s.onboarding);
  const { pending, error, run } = useAction();
  return (
    <Section title={t("settings.setup")}>
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
        <span className="text-small text-text-muted">{t("settings.setup.again")}</span>
        <Button variant="secondary" size="sm" disabled={pending !== null} onClick={() => run("reset", () => rpc("onboarding.reset"))}>
          {t("settings.setup.run")}
        </Button>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
        <span className="text-small text-text-muted">
          {onboarding?.safety_acknowledged ? t("settings.setup.accepted") : t("settings.setup.locked")}
        </span>
        {onboarding?.safety_acknowledged && (
          <Button variant="danger" size="sm" disabled={pending !== null} onClick={() => run("revoke", () => rpc("safety.revoke"))}>
            {t("settings.setup.withdraw")}
          </Button>
        )}
      </div>
      {error && <InlineError>{error}</InlineError>}
    </Section>
  );
}

/** Verified release updates: check, download, install (Setup asks for UAC). */
function Updates() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const { pending, error, run } = useAction();
  const desktop = useDaemon((s) => s.mode) === "tauri";
  const act = (action: "check" | "download" | "install") =>
    run(action, async () => {
      const tr = await transport();
      if (!tr.update) throw new Error(t("settings.updates.unavailable"));
      const next = await tr.update(action);
      if (next) setInfo(next);
    });
  return (
    <Section title={t("settings.updates")}>
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
        <span className="flex items-center gap-3 text-small text-text-muted">
          <Download className="size-4 shrink-0 text-text-subtle" aria-hidden />
          {info === null
            ? t("settings.updates.hint")
            : info.available
              ? t("settings.updates.available", {
                  version: info.available,
                  current: info.current,
                  size: info.size ? `, ${(info.size / 1048576).toFixed(1)} MB` : "",
                })
              : t("settings.updates.upToDate", { current: info.current })}
        </span>
        <span className="flex gap-2">
          <Button variant="secondary" size="sm" disabled={!desktop || pending !== null} onClick={() => act("check")}>
            {pending === "check" ? t("settings.updates.checking") : t("settings.updates.check")}
          </Button>
          {info?.available && !info.downloaded && (
            <Button size="sm" variant="primary" disabled={pending !== null} onClick={() => act("download")}>
              {pending === "download" ? t("settings.updates.downloading") : t("settings.updates.download")}
            </Button>
          )}
          {info?.downloaded && (
            <Button size="sm" variant="primary" disabled={pending !== null} onClick={() => act("install")}>
              {t("settings.updates.install")}
            </Button>
          )}
        </span>
      </div>
      {error && <InlineError>{error}</InlineError>}
    </Section>
  );
}
