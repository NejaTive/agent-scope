import { tr } from "@/i18n";
import type { Preferences, Product } from "@/types/config";
import { PRODUCT_DIR_LABEL } from "./labels";

export const TOOLING_PROVIDERS = [
  { id: "claude" as const, label: "Claude Code", prefKey: "trackClaude" as const },
  { id: "cursor" as const, label: "Cursor", prefKey: "trackCursor" as const },
] as const;

export type ToolingProviderId = (typeof TOOLING_PROVIDERS)[number]["id"];

export function isProductTracked(prefs: Preferences, product: Product): boolean {
  return product === "cursor" ? prefs.trackCursor : prefs.trackClaude;
}

export function trackedProviders(prefs: Preferences) {
  return TOOLING_PROVIDERS.filter((p) => prefs[p.prefKey]);
}

/** "Claude Code", "Cursor", or "Claude Code и Cursor". */
export function trackedProductLabels(prefs: Preferences): string {
  const labels = trackedProviders(prefs).map((p) => p.label);
  if (labels.length === 0) return tr("dashboard.emptyProductsFallback");
  if (labels.length === 1) return labels[0]!;
  return `${labels.slice(0, -1).join(", ")} ${tr("common.and")} ${labels[labels.length - 1]}`;
}

/** ".claude", ".cursor", or ".claude / .cursor". */
export function trackedDirLabels(prefs: Preferences): string {
  const dirs = trackedProviders(prefs).map((p) => PRODUCT_DIR_LABEL[p.id]);
  if (dirs.length === 0) return tr("memory.emptyDirsFallback");
  if (dirs.length === 1) return dirs[0]!;
  return dirs.join(" / ");
}

/** Join `dir/segment` for each enabled product, e.g. `.claude/rules и .cursor/rules`. */
export function trackedPathHint(prefs: Preferences, segment: string): string {
  const parts = trackedProviders(prefs).map(
    (p) => `${PRODUCT_DIR_LABEL[p.id]}/${segment}`,
  );
  if (parts.length === 0) return segment;
  if (parts.length === 1) return parts[0]!;
  return parts.join(` ${tr("common.and")} `);
}
