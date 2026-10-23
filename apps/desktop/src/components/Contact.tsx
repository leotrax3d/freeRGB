import { ExternalLink, Mail, Play } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { openExternal, type ExternalDestination } from "@/lib/api/transport";
export function Contact() {
  const [error, setError] = useState<string | null>(null);
  const open = async (destination: ExternalDestination) => {
    setError(null);
    try { await openExternal(destination); } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  };
  return <div className="flex flex-col gap-3">
    <p className="text-small text-text-muted">Contact the freeRGB Team or follow the project on YouTube.</p>
    <div className="flex flex-wrap gap-2">
      <Button variant="secondary" onClick={() => void open({ kind: "email" })}><Mail className="size-4" />freergbteam@gmail.com</Button>
      <Button variant="secondary" onClick={() => void open({ kind: "youtube" })}><Play className="size-4" />YouTube channel<ExternalLink className="size-3" /></Button>
    </div>
    {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
  </div>;
}
