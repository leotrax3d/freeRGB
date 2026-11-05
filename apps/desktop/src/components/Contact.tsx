import { ExternalLink, Mail, Play } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { InlineError } from "@/components/status";
import { openExternal, type ExternalDestination } from "@/lib/api/transport";
import { t } from "@/lib/i18n";
export function Contact() {
  const [error, setError] = useState<string | null>(null);
  const open = async (destination: ExternalDestination) => {
    setError(null);
    try { await openExternal(destination); } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  };
  return <div className="flex flex-col gap-3">
    <p className="text-small text-text-muted">{t("contact.intro")}</p>
    <div className="flex flex-wrap gap-2">
      <Button variant="secondary" onClick={() => void open({ kind: "email" })}><Mail className="size-4" />freergbteam@gmail.com</Button>
      <Button variant="secondary" onClick={() => void open({ kind: "youtube" })}><Play className="size-4" />{t("contact.youtube")}<ExternalLink className="size-3" aria-hidden /></Button>
    </div>
    {error && <InlineError>{error}</InlineError>}
  </div>;
}
