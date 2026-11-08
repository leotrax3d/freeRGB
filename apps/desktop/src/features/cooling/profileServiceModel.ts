import { t } from "./messages";

/** Parse locally; the daemon validates schema and hardware IDs. */
export function profileDocument(text: string): Record<string, unknown> {
  try {
    const value: unknown = JSON.parse(text);
    if (value && typeof value === "object" && !Array.isArray(value)) return value as Record<string, unknown>;
  } catch { /* Report a localized validation error below. */ }
  throw new Error(t("cooling.profileService.invalid"));
}
