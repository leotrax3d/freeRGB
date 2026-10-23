import { useEffect, useId, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { openExternal } from "@/lib/api/transport";
import { rpc } from "@/lib/store/daemon";
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
    try { await openExternal({ kind: "issue", title: report.title, body }); setNotice("GitHub opened. Review and submit the issue there. freeRGB has not sent the report."); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };
  const copy = async () => {
    try { await navigator.clipboard.writeText(body); setNotice("Report copied."); }
    catch { setError("Could not copy automatically. Select and copy the report text below."); }
  };
  const tooLong = new TextEncoder().encode(body).length > 6000;
  return <dialog ref={dialog} onCancel={(e) => { e.preventDefault(); onClose(); }} aria-labelledby={titleId} className="m-auto w-[min(92vw,760px)] max-h-[90vh] overflow-y-auto rounded-panel border border-border bg-surface p-6 text-text shadow-xl backdrop:bg-black/70">
    <h2 id={titleId} className="text-title font-semibold">Review error report{report ? ` · ${report.code}` : ""}</h2>
    <p className="mt-2 text-small text-text-muted">Includes the error code, app/effect versions, operating system and reduced device information. Device names, serial numbers, paths, sensor readings and rating credentials are excluded.</p>
    <p className="mt-2 text-small text-text-muted">You can edit or remove information before sharing. GitHub issues are public; submitting there requires a GitHub account. No report is sent automatically.</p>
    {!report && !error && <p role="status" className="mt-4 text-small">Preparing preview…</p>}
    {report && <label className="mt-4 flex flex-col gap-2 text-small">Report preview<textarea aria-label="Error report preview" value={body} onChange={(e) => setBody(e.target.value)} rows={15} className="w-full rounded-control border border-border bg-surface-sunken p-3 font-mono text-caption" /></label>}
    {tooLong && <p role="alert" className="mt-2 text-small text-danger-text">Shorten the report or copy it into GitHub manually.</p>}
    {error && <p role="alert" className="mt-3 text-small text-danger-text">{error}</p>}
    {notice && <p role="status" className="mt-3 text-small text-accent">{notice}</p>}
    <div className="mt-5 flex flex-wrap justify-end gap-2"><Button variant="ghost" onClick={onClose}>Close</Button><Button variant="secondary" disabled={!report} onClick={() => void copy()}>Copy report</Button><Button disabled={!report || busy || tooLong} onClick={() => void open()}>Open GitHub issue</Button></div>
  </dialog>;
}
