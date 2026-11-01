import { getLocale } from "@/i18n";

/** Russian plural forms: one / few (2–4) / many (0, 5–20, …). */
export type RuForms = readonly [one: string, few: string, many: string];

/** English plural forms: one / other. */
export type EnForms = readonly [one: string, other: string];

export function pluralRu(n: number, forms: RuForms): string {
  const abs = Math.abs(Math.trunc(n)) % 100;
  const dig = abs % 10;
  if (abs > 10 && abs < 20) return forms[2];
  if (dig === 1) return forms[0];
  if (dig >= 2 && dig <= 4) return forms[1];
  return forms[2];
}

export function pluralEn(n: number, forms: EnForms): string {
  return Math.abs(Math.trunc(n)) === 1 ? forms[0] : forms[1];
}

/** `"3 проекта"` — number + correctly declined noun/phrase. */
export function pluralCount(n: number, forms: RuForms): string {
  return `${n} ${pluralRu(n, forms)}`;
}

export function pluralCountEn(n: number, forms: EnForms): string {
  return `${n} ${pluralEn(n, forms)}`;
}

export type PluralKey =
  | "artifact"
  | "project"
  | "key"
  | "skill"
  | "rule"
  | "agent"
  | "hook"
  | "mcpServer"
  | "newChange"
  | "recentChange";

export const RU = {
  artifact: ["артефакт", "артефакта", "артефактов"],
  project: ["проект", "проекта", "проектов"],
  key: ["ключ", "ключа", "ключей"],
  skill: ["скилл", "скилла", "скиллов"],
  rule: ["правило", "правила", "правил"],
  agent: ["агент", "агента", "агентов"],
  hook: ["хук", "хука", "хуков"],
  mcpServer: ["MCP-сервер", "MCP-сервера", "MCP-серверов"],
  newChange: ["новое изменение", "новых изменения", "новых изменений"],
  recentChange: ["недавнее изменение", "недавних изменения", "недавних изменений"],
} as const satisfies Record<PluralKey, RuForms>;

export const EN = {
  artifact: ["artifact", "artifacts"],
  project: ["project", "projects"],
  key: ["key", "keys"],
  skill: ["skill", "skills"],
  rule: ["rule", "rules"],
  agent: ["agent", "agents"],
  hook: ["hook", "hooks"],
  mcpServer: ["MCP server", "MCP servers"],
  newChange: ["new change", "new changes"],
  recentChange: ["recent change", "recent changes"],
} as const satisfies Record<PluralKey, EnForms>;

/** Locale-aware `"3 changes"` / `"3 изменения"` using current i18n locale. */
export function pluralCountLocale(n: number, key: PluralKey): string {
  if (getLocale() === "en") {
    return pluralCountEn(n, EN[key]);
  }
  return pluralCount(n, RU[key]);
}

/** Forms for the current locale (Agent 2 / callers that pick forms then call plural*). */
export function formsFor(key: PluralKey): RuForms | EnForms {
  return getLocale() === "en" ? EN[key] : RU[key];
}
