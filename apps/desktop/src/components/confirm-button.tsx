import { useEffect, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/button";

/**
 * Two-step destructive button: the first click arms it, the second confirms.
 * Disarms after a few seconds or when focus leaves, so nothing is deleted by
 * a stray double click on another row.
 */
export function ConfirmButton({
  label,
  confirmLabel = "Delete?",
  icon,
  disabled,
  onConfirm,
}: {
  label: string;
  confirmLabel?: string;
  icon: ReactNode;
  disabled?: boolean;
  onConfirm: () => void;
}) {
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    if (!armed) return;
    const timer = setTimeout(() => setArmed(false), 4000);
    return () => clearTimeout(timer);
  }, [armed]);
  return armed ? (
    <Button size="sm" variant="danger" disabled={disabled} onBlur={() => setArmed(false)} onClick={onConfirm} autoFocus>
      {confirmLabel}
    </Button>
  ) : (
    <Button size="icon" variant="ghost" aria-label={label} title={label} disabled={disabled} onClick={() => setArmed(true)}>
      {icon}
    </Button>
  );
}
