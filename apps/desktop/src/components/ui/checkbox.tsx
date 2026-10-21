import { Check } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/** App-styled checkbox with its label; Space toggles, as with a native one. */
export function Checkbox({
  checked,
  onChange,
  children,
  className,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  children: ReactNode;
  className?: string;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={cn("group inline-flex items-center gap-2 rounded-chip text-small text-text outline-none", className)}
    >
      <span
        className={cn(
          "grid size-4 shrink-0 place-items-center rounded-[5px] border transition-colors duration-(--duration-fast) group-focus-visible:outline-2 group-focus-visible:outline-offset-2 group-focus-visible:outline-info",
          checked ? "border-accent bg-accent text-black" : "border-border-strong bg-surface group-hover:border-text-subtle",
        )}
      >
        {checked && <Check className="size-3" strokeWidth={3.5} />}
      </span>
      {children}
    </button>
  );
}
