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
  configured: boolean; by_effect: Record<string, { average: number; count: number }>; mine: Record<string, number>;
  updated_at: number | null; loading: boolean; saving: string | null; error: string | null;
}
