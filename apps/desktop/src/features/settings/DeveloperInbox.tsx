import { useState } from "react";
import { Section } from "@/components/page";
import { InlineError } from "@/components/status";
import { Button } from "@/components/ui/button";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { t } from "./messages";

type Submission = { id: string; kind: string; status: string; payload: Record<string, unknown>; created_at: string };
type Message = { id: string; text: string; created_at: string; from_developer?: boolean };
type Page = { items: Submission[]; next: string | null };
type Thread = { submission: Submission; messages: Message[] };

const statusLabel = (status: string) =>
  status === "new" || status === "in_progress" || status === "resolved" || status === "spam" ? t(`settings.inbox.status.${status}`) : status;

export function DeveloperInbox() {
  const mode = useDaemon((s) => s.mode);
  const [page, setPage] = useState<Page | null>(null);
  const [thread, setThread] = useState<Thread | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [requestId, setRequestId] = useState(crypto.randomUUID());
  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const summary = (s: Submission) => (typeof s.payload.message === "string" ? s.payload.message : s.kind);
  return (
    <Section title={t("settings.inbox.title")}>
      <div className="flex flex-col gap-3">
        <div className="flex gap-2">
          <Button
            variant="secondary"
            disabled={busy || mode !== "tauri"}
            onClick={() =>
              void run(async () => {
                setPage(await rpc<Page>("community.inbox"));
                if (thread) setThread(await rpc<Thread>("community.thread", { submission_id: thread.submission.id }));
              })
            }
          >
            {busy ? t("settings.inbox.loading") : t("settings.inbox.check")}
          </Button>
          {page?.next && (
            <Button variant="ghost" disabled={busy} onClick={() => void run(async () => setPage(await rpc<Page>("community.inbox", { cursor: page.next })))}>
              {t("settings.inbox.older")}
            </Button>
          )}
        </div>
        {page?.items.length === 0 && <p className="text-small text-text-subtle">{t("settings.inbox.empty")}</p>}
        {page?.items.map((item) => (
          <button
            key={item.id}
            disabled={busy}
            className="flex items-center justify-between gap-3 rounded-control border border-border bg-surface-sunken p-3 text-left text-small"
            onClick={() =>
              void run(async () => {
                setThread(await rpc<Thread>("community.thread", { submission_id: item.id }));
                setText("");
              })
            }
          >
            <span className="truncate">{summary(item)}</span>
            <span className="shrink-0 text-caption text-text-subtle">{statusLabel(item.status)}</span>
          </button>
        ))}
        {thread && (
          <div className="flex flex-col gap-3 rounded-card border border-border p-4">
            <h3 className="font-medium">{t("settings.inbox.conversation")}</h3>
            <p className="whitespace-pre-wrap break-words text-small text-text-muted">{summary(thread.submission)}</p>
            {thread.messages.map((message) => (
              <div key={message.id} className="rounded-control bg-surface-raised p-3">
                <p className="text-caption text-text-subtle">
                  {message.from_developer ? t("settings.inbox.team") : t("settings.inbox.you")} · {new Date(message.created_at).toLocaleString()}
                </p>
                <p className="whitespace-pre-wrap break-words text-small">{message.text}</p>
              </div>
            ))}
            <label className="flex flex-col gap-2 text-small">
              {t("settings.inbox.reply")}
              <textarea
                aria-label={t("settings.inbox.replyAria")}
                rows={3}
                maxLength={6000}
                value={text}
                onChange={(e) => {
                  setText(e.target.value);
                  setRequestId(crypto.randomUUID());
                }}
                className="rounded-control border border-border bg-surface-sunken p-3"
                disabled={busy}
              />
            </label>
            <Button
              disabled={busy || !text.trim()}
              onClick={() =>
                void run(async () => {
                  await rpc("community.reply", { submission_id: thread.submission.id, text: text.trim(), request_id: requestId });
                  setText("");
                  setRequestId(crypto.randomUUID());
                  setThread(await rpc<Thread>("community.thread", { submission_id: thread.submission.id }));
                })
              }
            >
              {t("settings.inbox.send")}
            </Button>
          </div>
        )}
        {error && <InlineError>{error}</InlineError>}
      </div>
    </Section>
  );
}
