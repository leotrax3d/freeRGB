import * as S from "@radix-ui/react-select";
import { Check, ChevronDown } from "lucide-react";
import { t } from "@/lib/i18n";
import { cn } from "@/lib/utils";

export interface Option<T extends string> {
  value: T;
  label: string;
  disabled?: boolean;
}

/**
 * App-styled replacement for native <select>: same dark surface, radius and
 * focus ring as the rest of the UI, full keyboard support via Radix.
 */
export function Select<T extends string>({
  value,
  options,
  onChange,
  label,
  placeholder = t("common.choose"),
  className,
}: {
  value: T | "";
  options: readonly Option<T>[];
  onChange: (value: T) => void;
  label: string;
  placeholder?: string;
  className?: string;
}) {
  return (
    <S.Root value={value || undefined} onValueChange={(v) => onChange(v as T)}>
      <S.Trigger
        aria-label={label}
        className={cn(
          "inline-flex h-8 min-w-0 items-center justify-between gap-2 rounded-chip border border-border-strong bg-surface px-2.5 text-small text-text outline-none transition-colors hover:bg-surface-raised focus-visible:border-info data-[placeholder]:text-text-subtle",
          className,
        )}
      >
        <span className="truncate">
          <S.Value placeholder={placeholder} />
        </span>
        <S.Icon>
          <ChevronDown className="size-3.5 shrink-0 text-text-subtle" />
        </S.Icon>
      </S.Trigger>
      <S.Portal>
        <S.Content
          position="popper"
          sideOffset={4}
          collisionPadding={8}
          className="z-50 max-h-[min(320px,var(--radix-select-content-available-height))] min-w-(--radix-select-trigger-width) overflow-hidden rounded-control bg-surface-raised p-1 shadow-popover animate-[pop-in_var(--duration-fast)_var(--ease-out)]"
        >
          <S.Viewport>
            {options.map((o) => (
              <S.Item
                key={o.value}
                value={o.value}
                disabled={o.disabled}
                className="relative flex h-8 cursor-pointer select-none items-center rounded-chip pl-7 pr-3 text-small text-text-muted outline-none data-[disabled]:pointer-events-none data-[disabled]:opacity-40 data-[highlighted]:bg-white/8 data-[highlighted]:text-text data-[state=checked]:text-text"
              >
                <S.ItemIndicator className="absolute left-2">
                  <Check className="size-3.5 text-accent" />
                </S.ItemIndicator>
                <S.ItemText>{o.label}</S.ItemText>
              </S.Item>
            ))}
          </S.Viewport>
        </S.Content>
      </S.Portal>
    </S.Root>
  );
}
