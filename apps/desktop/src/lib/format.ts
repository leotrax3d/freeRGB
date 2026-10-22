// Numbers follow the UI language (English) so "1,165" never reads as 1.165.
const LOCALE = "en-US";
const cache = new Map<number, Intl.NumberFormat>();

export function num(value: number, maxDigits = 0): string {
  let f = cache.get(maxDigits);
  if (!f) cache.set(maxDigits, (f = new Intl.NumberFormat(LOCALE, { maximumFractionDigits: maxDigits })));
  return f.format(value);
}

export function duration(seconds: number): string {
  const s = Math.floor(seconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h} h ${m} min`;
  if (m > 0) return `${m} min ${s % 60} s`;
  return `${s} s`;
}
