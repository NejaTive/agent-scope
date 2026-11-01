import { tr } from "@/i18n";
import type { Preferences, Product, ProjectInfo } from "@/types/config";
import { PRODUCT_DIR_LABEL } from "./labels";

/**
 * Registry of AI tooling products. Add a provider here + backend `Tooling` /
 * `track*` pref + scanner — avoid per-product copy-paste in views.
 * Future stubs (no full scanner): windsurf, continue, gemini-cli.
 */
export const TOOLING_PROVIDERS = [
  { id: "claude" as const, label: "Claude Code", prefKey: "trackClaude" as const },
  { id: "cursor" as const, label: "Cursor", prefKey: "trackCursor" as const },
  { id: "codex" as const, label: "Codex", prefKey: "trackCodex" as const },
] as const;

export type ToolingProviderId = (typeof TOOLING_PROVIDERS)[number]["id"];
export type ToolingPrefKey = (typeof TOOLING_PROVIDERS)[number]["prefKey"];

export function prefKeyForProduct(product: Product): ToolingPrefKey | undefined {
  return TOOLING_PROVIDERS.find((p) => p.id === product)?.prefKey;
}

export function isProductTracked(prefs: Preferences, product: Product): boolean {
  const key = prefKeyForProduct(product);
  return key ? prefs[key] : false;
}

/** True when at least one registered product is enabled. */
export function hasTrackedProduct(prefs: Preferences): boolean {
  return TOOLING_PROVIDERS.some((p) => prefs[p.prefKey]);
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

/** Project has a product's config directory (`.claude`, `.cursor`, `.codex`). */
export function projectHasProductDir(p: ProjectInfo, product: Product): boolean {
  switch (product) {
    case "cursor":
      return !!p.hasCursorDir;
    case "codex":
      return !!p.hasCodexDir;
    default:
      return !!p.hasClaudeDir;
  }
}

/** Project has any marker file/dir for the product (dir, CLAUDE.md, MCP json). */
export function projectHasProduct(p: ProjectInfo, product: Product): boolean {
  switch (product) {
    case "cursor":
      return p.hasCursorDir || p.hasCursorMcpJson;
    case "codex":
      return !!p.hasCodexDir;
    default:
      return p.hasClaudeDir || p.hasClaudeMd || p.hasMcpJson;
  }
}
