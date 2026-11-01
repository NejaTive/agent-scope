import { tr } from "@/i18n";
import type {
  ArtifactKind,
  DiscoveryStatus,
  Product,
  Scope,
} from "@/types/config";

/** Display names — brand strings must stay untranslated. */
export const PRODUCT_LABEL: Record<Product, string> = {
  claude: "Claude Code",
  cursor: "Cursor",
  codex: "Codex",
};

export const PRODUCT_ORDER: Product[] = ["claude", "cursor", "codex"];

/** Dir badge text (filesystem names, not translated). */
export const PRODUCT_DIR_LABEL: Record<Product, string> = {
  claude: ".claude",
  cursor: ".cursor",
  codex: ".codex",
};

/** Missing / null product → Claude Code for backward compatibility. */
export function resolveProduct(product?: Product | null): Product {
  if (product === "cursor" || product === "codex") return product;
  return "claude";
}

/** Locale-aware record: each access resolves via vue-i18n. */
function i18nRecord<K extends string>(
  keys: readonly K[],
  toKey: (k: K) => string,
): Record<K, string> {
  const out = {} as Record<K, string>;
  for (const k of keys) {
    Object.defineProperty(out, k, {
      enumerable: true,
      configurable: true,
      get: () => tr(toKey(k)),
    });
  }
  return out;
}

const SCOPE_KEYS = [
  "managed",
  "user",
  "project",
  "projectLocal",
  "plugin",
  "nested",
  "globalConfig",
] as const satisfies readonly Scope[];

const KIND_KEYS = [
  "skill",
  "command",
  "rule",
  "agent",
  "hook",
  "mcpServer",
  "plugin",
  "settings",
  "memory",
  "claudeMd",
  "workflow",
  "outputStyle",
  "other",
] as const satisfies readonly ArtifactKind[];

const STATUS_KEYS = [
  "present",
  "missing",
  "unreadable",
  "invalidJson",
  "invalidFrontmatter",
  "partial",
] as const satisfies readonly DiscoveryStatus[];

export const SCOPE_LABEL: Record<Scope, string> = i18nRecord(
  SCOPE_KEYS,
  (k) => `labels.scope.${k}`,
);

export const SCOPE_ORDER: Scope[] = [
  "managed",
  "user",
  "project",
  "projectLocal",
  "nested",
  "plugin",
  "globalConfig",
];

/** Precedence (higher wins) used for display ordering only. */
export const SCOPE_PRECEDENCE: Record<Scope, number> = {
  managed: 6,
  projectLocal: 5,
  project: 4,
  nested: 3,
  user: 2,
  plugin: 1,
  globalConfig: 0,
};

export const KIND_LABEL: Record<ArtifactKind, string> = i18nRecord(
  KIND_KEYS,
  (k) => `labels.kind.${k}`,
);

export const KIND_PLURAL: Record<ArtifactKind, string> = i18nRecord(
  KIND_KEYS,
  (k) => `labels.kindPlural.${k}`,
);

export const STATUS_LABEL: Record<DiscoveryStatus, string> = i18nRecord(
  STATUS_KEYS,
  (k) => `labels.discovery.${k}`,
);

export const STATUS_TONE: Record<
  DiscoveryStatus,
  "ok" | "warn" | "danger" | "neutral"
> = {
  present: "ok",
  missing: "neutral",
  unreadable: "danger",
  invalidJson: "danger",
  invalidFrontmatter: "warn",
  partial: "warn",
};

/** Route for each artifact kind (used by global search). */
export function routeForKind(kind: ArtifactKind): string {
  switch (kind) {
    case "skill":
    case "command":
      return "/skills";
    case "rule":
      return "/rules";
    case "agent":
      return "/agents";
    case "hook":
      return "/hooks";
    case "mcpServer":
      return "/mcp";
    case "plugin":
      return "/plugins";
    case "settings":
      return "/settings";
    case "memory":
    case "claudeMd":
    case "workflow":
    case "outputStyle":
      return "/memory";
    default:
      return "/global";
  }
}
