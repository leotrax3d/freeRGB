import { useId, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { InlineError } from "@/components/status";
import { t } from "@/lib/i18n";

/** Mirrors `profiles::validate_name` in the daemon (which stays authoritative). */
export function nameProblem(name: string): string | null {
  if (!name) return null;
  if (name.trim() !== name) return t("name.trim");
  if ([...name].length > 64) return t("name.length");
  if (!/^[\p{L}\p{N} _-]+$/u.test(name)) return t("name.chars");
  return null;
}

/** Single-line name input with a submit button; warns before overwriting. */
export function NameForm({
  label,
  placeholder,
  submit,
  existing,
  busy,
  onSubmit,
}: {
  label: string;
  placeholder: string;
  submit: string;
  existing: readonly string[];
  busy: boolean;
  onSubmit: (name: string) => Promise<boolean>;
}) {
  const id = useId();
  const [name, setName] = useState("");
  const problem = nameProblem(name);
  const overwrites = !problem && existing.includes(name);
  const send = async (e: FormEvent) => {
    e.preventDefault();
    if (!name || problem) return;
    if (await onSubmit(name)) setName("");
  };
  return (
    <form onSubmit={send} className="flex flex-col gap-1.5">
      <label htmlFor={id} className="sr-only">
        {label}
      </label>
      <div className="flex gap-2">
        <input
          id={id}
          value={name}
          placeholder={placeholder}
          maxLength={64}
          autoComplete="off"
          onChange={(e) => setName(e.target.value)}
          aria-invalid={!!problem}
          className="h-9 min-w-0 flex-1 rounded-control border border-border-strong bg-surface-sunken px-3 text-body text-text outline-none placeholder:text-text-disabled focus-visible:border-info aria-invalid:border-danger"
        />
        <Button type="submit" variant="primary" disabled={!name || !!problem || busy}>
          {overwrites ? t("name.replace") : submit}
        </Button>
      </div>
      {problem && <InlineError>{problem}</InlineError>}
      {overwrites && <p role="status" className="text-caption text-[#fbbf24]">{t("name.exists", { name })}</p>}
    </form>
  );
}
