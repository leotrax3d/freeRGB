import { Sparkles, Trash2 } from "lucide-react";
import { InlineError } from "@/components/status";
import { t } from "./messages";
import { ConfirmButton } from "@/components/confirm-button";
import { Section } from "@/components/page";
import type { EffectSpec } from "@/lib/api/types";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { cn } from "@/lib/utils";

/** Saved custom effects: apply with one click, delete with a confirmed click. */
export function CustomEffects() {
  const names = useDaemon((s) => s.custom_effects);
  const { pending, error, run } = useAction();
  if (names === null) return null; // daemon without storage (--no-state)
  if (names === undefined) return null; // first poll after opening the view

  const apply = (name: string) =>
    run(name, async () => {
      const spec = await rpc<EffectSpec>("custom_effects.get", { name });
      await rpc("effects.set", spec);
    });

  return (
    <Section title={t("lighting.custom")} aside={error && <InlineError>{error}</InlineError>}>
      {names.length === 0 ? (
        <p className="text-small text-text-subtle">{t("lighting.custom.empty")}</p>
      ) : (
        <ul className="flex flex-wrap gap-2">
          {names.map((name) => (
            <li
              key={name}
              className={cn(
                "group flex items-center gap-1 rounded-control bg-surface-sunken py-1 pl-1 pr-1 transition-colors duration-(--duration-fast) hover:bg-surface-raised",
                pending === name && "opacity-60",
              )}
            >
              <button
                type="button"
                onClick={() => apply(name)}
                disabled={pending !== null}
                className="flex h-8 items-center gap-2 rounded-chip px-2 text-body"
              >
                <Sparkles className="size-4 text-creative-text" aria-hidden />
                {name}
              </button>
              <ConfirmButton
                label={t("lighting.custom.delete", { name })}
                icon={<Trash2 />}
                disabled={pending !== null}
                onConfirm={() => run(name, () => rpc("custom_effects.delete", { name }), t("lighting.custom.deleted", { name }))}
              />
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
