import type { EffectSpec } from "@/lib/api/types";
export interface Parameter { key: string; label: string; min: number; max: number; default: number; step: number }
export interface Package {
  publisher: { id: string; name: string };
  id: string; version: string; api: number; name: string; description: string; category: string;
  preview: [string, string]; permissions: string[]; parameters: Parameter[];
  artifact: { target: string; url: string; sha256: string; size: number };
}
export interface MarketplaceStatus {
  packages: {
    source: string; catalog: Package[]; installed: Package[]; error: string | null;
    failure: { code: string; effect_id: string | null; version: string | null } | null;
    ratings: Ratings;
    job: { action: string; id: string | null; stage: string; bytes: number; total: number } | null;
  };
  active: EffectSpec;
  runtime_error: string | null;
  runtime_error_code: string | null;
}
export function defaults(pkg: Package): Record<string, number> {
  return Object.fromEntries(pkg.parameters.map((p) => [p.key, p.default]));
}
export function matches(pkg: Package, query: string, installed: boolean, onlyInstalled: boolean): boolean {
  return (!onlyInstalled || installed) && `${pkg.name} ${pkg.description} ${pkg.category} ${pkg.publisher.name}`.toLowerCase().includes(query.trim().toLowerCase());
}

export interface Ratings {
  configured: boolean; by_effect: Record<string, { average: number; count: number; downloads?: number }>; mine: Record<string, number>;
  updated_at: number | null; loading: boolean; saving: string | null; error: string | null;
}

export type SortOrder = "downloads" | "rating" | "name" | "installed" | "updates";
export function sortPackages(packages: Package[], order: SortOrder, ratings: Ratings["by_effect"]): Package[] {
  return [...packages].sort((a, b) => {
    const left = ratings[a.id], right = ratings[b.id];
    const score = order === "downloads" ? (right?.downloads ?? 0) - (left?.downloads ?? 0)
      : order === "rating" ? (right?.average ?? 0) - (left?.average ?? 0) || (right?.count ?? 0) - (left?.count ?? 0) : 0;
    return score || a.name.localeCompare(b.name) || a.id.localeCompare(b.id);
  });
}
