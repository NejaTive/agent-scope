/** Russian plural forms: one / few (2–4) / many (0, 5–20, …). */
export type RuForms = readonly [one: string, few: string, many: string];

export function pluralRu(n: number, forms: RuForms): string {
  const abs = Math.abs(Math.trunc(n)) % 100;
  const dig = abs % 10;
  if (abs > 10 && abs < 20) return forms[2];
  if (dig === 1) return forms[0];
  if (dig >= 2 && dig <= 4) return forms[1];
  return forms[2];
}

/** `"3 проекта"` — number + correctly declined noun/phrase. */
export function pluralCount(n: number, forms: RuForms): string {
  return `${n} ${pluralRu(n, forms)}`;
}

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
} as const satisfies Record<string, RuForms>;
