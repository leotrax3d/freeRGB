import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import type { MacroDefinition } from "@/lib/api/control";
import { rpc } from "@/lib/store/daemon";
import { RecordingSession, recordedKey } from "./recording";

export function RecordingPanel({ onDraft }: { onDraft: (draft: MacroDefinition) => void }) {
  const [active, setActive] = useState(false);
  const [message, setMessage] = useState("");
  const [textMode, setTextMode] = useState(false);
  const [textValue, setTextValue] = useState("");
  const pad = useRef<HTMLDivElement>(null);
  const text = useRef<HTMLTextAreaElement>(null);
  const root = useRef<HTMLElement>(null);
  const session = useRef<RecordingSession | null>(null);
  const discard = () => {
    const recording = session.current;
    session.current = null;
    setActive(false);
    setTextValue("");
    if (recording) void recording.stop(true).catch(() => {});
  };
  const event = (value: object) => {
    void session.current?.event(value).catch((e: unknown) => {
      setMessage(String(e)); discard();
    });
  };
  const review = async () => {
    const recording = session.current;
    session.current = null;
    try {
      if (textMode && textValue) await recording?.event({ type: "text", text: textValue });
      const draft = await recording?.stop(false);
      if (draft) onDraft(draft);
      setMessage("Review the draft, then save it explicitly.");
    }
    catch (e) { void recording?.stop(true).catch(() => {}); setMessage(String(e)); }
    setActive(false);
  };
  useEffect(() => {
    if (!active) return;
    (textMode ? text.current : pad.current)?.focus();
    const lost = () => { discard(); setMessage("Recording discarded after focus loss."); };
    const hidden = () => { if (document.hidden) lost(); };
    const focus = (e: FocusEvent) => {
      if (e.target instanceof HTMLButtonElement && root.current?.contains(e.target)) return;
      if (e.target !== pad.current && e.target !== text.current) lost();
    };
    window.addEventListener("blur", lost);
    document.addEventListener("visibilitychange", hidden);
    document.addEventListener("focusin", focus);
    const timer = window.setInterval(() => {
      if (!document.hasFocus()) { lost(); return; }
      event({ type: "heartbeat" });
      void rpc<{ recording: boolean }>("macros.record.status").then((s) => {
        if (!s.recording) { discard(); setMessage("Recording ended; the unsaved buffer was discarded."); }
      }).catch(lost);
    }, 750);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("blur", lost);
      document.removeEventListener("visibilitychange", hidden);
      document.removeEventListener("focusin", focus);
      discard();
    };
    // The session and its pad are fixed for each recording.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  return <section ref={root} className="mb-4 flex flex-col gap-2 rounded-card border border-border p-4" aria-label="Macro recording">
    <p role="status">{active ? "● Recording — unsaved, focused pad only (30 s maximum)" : "Record into the pad. Nothing is saved until you save the macro."}</p>
    <label><input type="checkbox" disabled={active} checked={textMode} onChange={(e) => setTextMode(e.target.checked)} /> Record text as one step when stopped</label>
    {active && (textMode ? <textarea ref={text} aria-label="Record text" autoComplete="off" spellCheck={false} value={textValue} onKeyDown={(e) => { if (e.key === "F12" && !e.nativeEvent.isComposing) { e.preventDefault(); void review(); } }} onChange={(e) => setTextValue(e.target.value)} onPaste={(e) => e.preventDefault()} /> :
      <div ref={pad} tabIndex={0} role="group" aria-label="Recording pad" className="rounded-control border border-info p-4"
        onKeyDown={(e) => { e.preventDefault(); if (e.key === "F12") { void review(); return; } if (!e.repeat) { const key = recordedKey(e.key); if (key) event({ type: "key_down", key }); } }}
        onKeyUp={(e) => { e.preventDefault(); const key = recordedKey(e.key); if (key) event({ type: "key_up", key }); }}>
        Type keys here. Focus changes discard recording. Use Stop or F12 to review.
      </div>)}
    <div className="flex gap-2">
      <Button disabled={active} onClick={async () => {
        const recording = new RecordingSession(rpc);
        try { await recording.start(); session.current = recording; setMessage(""); setTextValue(""); setActive(true); }
        catch (e) { setMessage(String(e)); }
      }}>Record</Button>
      <Button disabled={!active} onMouseDown={(e) => e.preventDefault()} onClick={() => void review()}>Stop and review</Button>
      <Button disabled={!active} onMouseDown={(e) => e.preventDefault()} onClick={discard}>Discard</Button>
    </div>
    {message && <p role="status">{message}</p>}
  </section>;
}
