import { useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { rpc, useDaemon } from "@/lib/store/daemon";

type Report = {kind: "feedback" | "hardware"; text: string; requestId: string};
export function DeveloperContact() {
  const mode = useDaemon((s) => s.mode);
  const [message, setMessage] = useState("");
  const [report, setReport] = useState<Report | null>(null);
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const prepare = async (kind: Report["kind"]) => {
    setError(null); setNotice(null); setPending(true);
    try {
      const payload = kind === "feedback" ? {message: message.trim()} : await rpc("diagnostics.export", {include_cooling_hardware: true});
      setReport({kind, text: kind === "feedback" ? message.trim() : JSON.stringify(payload, null, 2), requestId: crypto.randomUUID()});
    } catch(e) {setError(e instanceof Error ? e.message : String(e));}
    finally {setPending(false);}
  };
  const send = async () => {
    if(!report) return;
    setPending(true); setError(null);
    try {
      if (!report.text.trim()) throw new Error("Please enter a message before sending.");
      const payload: unknown = report.kind === "feedback" ? {message: report.text.trim()} : JSON.parse(report.text);
      if(!payload || typeof payload !== "object" || Array.isArray(payload)) throw new Error("The report must be a JSON object.");
      if(new TextEncoder().encode(report.text).length > 30000) throw new Error("The report must be smaller than 30 KB.");
      await rpc("community.submit", {kind: report.kind, payload, request_id: report.requestId, reviewed: true});
      setNotice(report.kind === "feedback" ? "Your message was sent to the freeRGB developers." : "Your hardware report was sent to the freeRGB developers.");
      if(report.kind === "feedback") setMessage("");
      setReport(null);
    } catch(e) {setError(e instanceof Error ? e.message : String(e));}
    finally {setPending(false);}
  };
  return <Section title="Contact the developers">
    <div className="flex flex-col gap-4">
      <p className="text-small text-text-muted">Send feedback or an optional hardware report to the freeRGB team. Only admins can read submissions. Nothing is sent until you review and confirm it.</p>
      <label className="flex flex-col gap-2 text-small">Your message<textarea aria-label="Message to the developers" value={message} onChange={(e) => {setMessage(e.target.value); setReport(null); setNotice(null);}} maxLength={12000} rows={5} placeholder="What would you like to tell us?" className="w-full rounded-control border border-border bg-surface-sunken p-3" disabled={pending} /></label>
      <div className="flex flex-wrap gap-3"><Button disabled={pending || !message.trim() || mode !== "tauri"} onClick={() => void prepare("feedback")}>Preview message</Button><Button variant="secondary" disabled={pending || mode !== "tauri"} onClick={() => void prepare("hardware")}>Preview hardware report</Button></div>
      {mode !== "tauri" && <p className="text-caption text-text-subtle">Sending reports is available in the desktop app.</p>}
      {report && <div className="flex flex-col gap-3 rounded-card border border-border p-4">
        <h3 className="font-medium">{report.kind === "feedback" ? "Review your message" : "Review your hardware report"}</h3>
        <p className="text-small text-text-muted">This is the exact content that will be sent. You can remove details before sending.{report.kind === "hardware" && " Hardware reports omit device names, serial numbers, paths and raw errors."}</p>
        <label className="flex flex-col gap-2 text-small">Report preview<textarea aria-label="Developer report preview" value={report.text} onChange={(e) => setReport({...report, text: e.target.value, requestId: crypto.randomUUID()})} rows={12} className="w-full rounded-control border border-border bg-surface-sunken p-3 font-mono text-caption" disabled={pending} /></label>
        <div className="flex gap-3"><Button disabled={pending} onClick={() => void send()}>{pending ? "Sending…" : "Send to developers"}</Button><Button variant="ghost" disabled={pending} onClick={() => setReport(null)}>Cancel</Button></div>
      </div>}
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
      {notice && <p role="status" className="text-small text-text-muted">{notice}</p>}
    </div>
  </Section>;
}
