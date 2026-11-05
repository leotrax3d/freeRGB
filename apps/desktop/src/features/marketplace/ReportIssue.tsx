import { useEffect, useId, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { openExternal } from "@/lib/api/transport";
import { rpc } from "@/lib/store/daemon";
import { InlineError } from "@/components/status";
import { t } from "./messages";
interface Prepared { code: string; title: string; body: string }
export function ReportIssue({ kind, onClose }: { kind: "runtime" | "operation"; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const [report, setReport] = useState<Prepared | null>(null);
  const [body, setBody] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    let stopped = false;
    void rpc<Prepared>("marketplace.report.prepare", { kind }).then((value) => {
      if (!stopped) { setReport(value); setBody(value.body); }
    }).catch((e: unknown) => { if (!stopped) setError(e instanceof Error ? e.message : String(e)); });
    return () => { stopped = true; element?.close(); };
  }, [kind]);
  const open = async () => {
    if (!report) return;
    setBusy(true); setError(null);
    try { await openExternal({ kind: "issue", title: report.title, body }); setNotice(t("market.report.opened")); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };
  const copy = async () => {
    try { await navigator.clipboard.writeText(body); setNotice(t("market.report.copied")); }
    catch { setError(t("market.report.copyFailed")); }
  };
  const tooLong = new TextEncoder().encode(body).length > 6000;
  return <dialog ref={dialog} onCancel={(e) => { e.preventDefault(); onClose(); }} aria-labelledby={titleId} className="m-auto w-[min(92vw,760px)] max-h-[90vh] overflow-y-auto rounded-panel border border-border bg-surface p-6 text-text shadow-xl backdrop:bg-black/70">
    <h2 id={titleId} className="text-title font-semibold">{t("market.report.title")}{report ? ` · ${report.code}` : ""}</h2>
    <p className="mt-2 text-small text-text-muted">{t("market.report.intro1")}</p>
    <p className="mt-2 text-small text-text-muted">{t("market.report.intro2")}</p>
    {!report && !error && <p role="status" className="mt-4 text-small">{t("market.report.preparing")}</p>}
    {report && <label className="mt-4 flex flex-col gap-2 text-small">{t("market.report.preview")}<textarea aria-label={t("market.report.previewAria")} value={body} onChange={(e) => setBody(e.target.value)} rows={15} className="w-full rounded-control border border-border bg-surface-sunken p-3 font-mono text-caption" /></label>}
    {tooLong && <InlineError className="mt-2">{t("market.report.tooLong")}</InlineError>}
    {error && <InlineError className="mt-3">{error}</InlineError>}
    {notice && <p role="status" className="mt-3 text-small text-accent">{notice}</p>}
    <div className="mt-5 flex flex-wrap justify-end gap-2"><Button variant="ghost" onClick={onClose}>{t("market.report.close")}</Button><Button variant="secondary" disabled={!report} onClick={() => void copy()}>{t("market.report.copy")}</Button><Button disabled={!report || busy || tooLong} onClick={() => void open()}>{t("market.report.open")}</Button></div>
  </dialog>;
}
