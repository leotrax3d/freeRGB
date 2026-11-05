import { useState } from "react";
import { Section } from "@/components/page";
import { InlineError } from "@/components/status";
import { Button } from "@/components/ui/button";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { t } from "./messages";

type Report = { kind: "feedback" | "hardware"; text: string; requestId: string };

export function DeveloperContact() {
  const mode = useDaemon((s) => s.mode);
  const [message, setMessage] = useState("");
  const [report, setReport] = useState<Report | null>(null);
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const prepare = async (kind: Report["kind"]) => {
    setError(null);
    setNotice(null);
    setPending(true);
    try {
      const payload = kind === "feedback" ? { message: message.trim() } : await rpc("diagnostics.export", { include_cooling_hardware: true });
      setReport({ kind, text: kind === "feedback" ? message.trim() : JSON.stringify(payload, null, 2), requestId: crypto.randomUUID() });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };
  const send = async () => {
    if (!report) return;
    setPending(true);
    setError(null);
    try {
      if (!report.text.trim()) throw new Error(t("settings.contact.errEmpty"));
      const payload: unknown = report.kind === "feedback" ? { message: report.text.trim() } : JSON.parse(report.text);
      if (!payload || typeof payload !== "object" || Array.isArray(payload)) throw new Error(t("settings.contact.errObject"));
      if (new TextEncoder().encode(report.text).length > 30000) throw new Error(t("settings.contact.errSize"));
      await rpc("community.submit", { kind: report.kind, payload, request_id: report.requestId, reviewed: true });
      setNotice(report.kind === "feedback" ? t("settings.contact.sentMessage") : t("settings.contact.sentHardware"));
      if (report.kind === "feedback") setMessage("");
      setReport(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };
  return (
    <Section title={t("settings.contact.title")}>
      <div className="flex flex-col gap-4">
        <p className="text-small text-text-muted">{t("settings.contact.intro")}</p>
        <label className="flex flex-col gap-2 text-small">
          {t("settings.contact.message")}
          <textarea
            aria-label={t("settings.contact.messageAria")}
            value={message}
            onChange={(e) => {
              setMessage(e.target.value);
              setReport(null);
              setNotice(null);
            }}
            maxLength={12000}
            rows={5}
            placeholder={t("settings.contact.placeholder")}
            className="w-full rounded-control border border-border bg-surface-sunken p-3"
            disabled={pending}
          />
        </label>
        <div className="flex flex-wrap gap-3">
          <Button disabled={pending || !message.trim() || mode !== "tauri"} onClick={() => void prepare("feedback")}>
            {t("settings.contact.previewMessage")}
          </Button>
          <Button variant="secondary" disabled={pending || mode !== "tauri"} onClick={() => void prepare("hardware")}>
            {t("settings.contact.previewHardware")}
          </Button>
        </div>
        {mode !== "tauri" && <p className="text-caption text-text-subtle">{t("settings.contact.desktopOnly")}</p>}
        {report && (
          <div className="flex flex-col gap-3 rounded-card border border-border p-4">
            <h3 className="font-medium">{report.kind === "feedback" ? t("settings.contact.reviewMessage") : t("settings.contact.reviewHardware")}</h3>
            <p className="text-small text-text-muted">
              {t("settings.contact.reviewText")}
              {report.kind === "hardware" && t("settings.contact.reviewHardwareNote")}
            </p>
            <label className="flex flex-col gap-2 text-small">
              {t("settings.contact.previewLabel")}
              <textarea
                aria-label={t("settings.contact.previewAria")}
                value={report.text}
                onChange={(e) => setReport({ ...report, text: e.target.value, requestId: crypto.randomUUID() })}
                rows={12}
                className="w-full rounded-control border border-border bg-surface-sunken p-3 font-mono text-caption"
                disabled={pending}
              />
            </label>
            <div className="flex gap-3">
              <Button disabled={pending} onClick={() => void send()}>
                {pending ? t("settings.contact.sending") : t("settings.contact.send")}
              </Button>
              <Button variant="ghost" disabled={pending} onClick={() => setReport(null)}>
                {t("settings.contact.cancel")}
              </Button>
            </div>
          </div>
        )}
        {error && <InlineError>{error}</InlineError>}
        {notice && <p role="status" className="text-small text-text-muted">{notice}</p>}
      </div>
    </Section>
  );
}
