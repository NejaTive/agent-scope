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
};

export const PRODUCT_ORDER: Product[] = ["claude", "cursor"];

/** Dir badge text (filesystem names, not translated). */
export const PRODUCT_DIR_LABEL: Record<Product, string> = {
  claude: ".claude",
  cursor: ".cursor",
};

/** Missing / null product → Claude Code for backward compatibility. */
export function resolveProduct(product?: Product | null): Product {
  return product === "cursor" ? "cursor" : "claude";
}

export const SCOPE_LABEL: Record<Scope, string> = {
  managed: "Управляемый",
  user: "Пользователь",
  project: "Проект",
  projectLocal: "Локальный",
  plugin: "Плагин",
  nested: "Вложенный",
  globalConfig: "Глобальные",
};

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

export const KIND_LABEL: Record<ArtifactKind, string> = {
  skill: "Скилл",
  command: "Команда",
  rule: "Правило",
  agent: "Агент",
  hook: "Хук",
  mcpServer: "MCP-сервер",
  plugin: "Плагин",
  settings: "Настройки",
  memory: "Память",
  claudeMd: "CLAUDE.md",
  workflow: "Воркфлоу",
  outputStyle: "Стиль вывода",
  other: "Прочее",
};

export const KIND_PLURAL: Record<ArtifactKind, string> = {
  skill: "Скиллы",
  command: "Команды",
  rule: "Правила",
  agent: "Агенты",
  hook: "Хуки",
  mcpServer: "MCP-серверы",
  plugin: "Плагины",
  settings: "Настройки",
  memory: "Память",
  claudeMd: "CLAUDE.md",
  workflow: "Воркфлоу",
  outputStyle: "Стили вывода",
  other: "Прочее",
};

export const STATUS_LABEL: Record<DiscoveryStatus, string> = {
  present: "Найден",
  missing: "Отсутствует",
  unreadable: "Нечитаемый",
  invalidJson: "Невалидный JSON",
  invalidFrontmatter: "Невалидный frontmatter",
  partial: "Частичный",
};

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
