<script setup lang="ts">
import { computed } from "vue";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import { maskRecord } from "@/lib/secrets";
import type { McpServerEntry } from "@/types/config";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList, { type Field } from "@/components/common/FieldList.vue";
import Badge from "@/components/common/Badge.vue";

const scan = useScanStore();
const prefs = usePreferencesStore();
const byId = computed(() => new Map(scan.mcpServers.map((s) => [s.id, s])));

const items = computed<BrowserItem[]>(() =>
  scan.mcpServers.map((s) => ({
    id: s.id,
    name: s.name,
    subtitle: s.url ?? ([s.command, ...(s.args ?? [])].filter(Boolean).join(" ") || null),
    scope: s.scope,
    status: s.status,
    mtime: s.mtime,
    path: s.sourcePath,
    projectPath: s.projectPath,
    artifactId: s.sourcePath,
    kind: "mcpServer",
    product: s.product,
    searchText: [s.serverType, s.origin, s.command, s.url].filter(Boolean).join(" "),
  })),
);

function connectionFields(s: McpServerEntry): Field[] {
  return [
    { label: "Тип", value: s.serverType },
    { label: "Источник", value: s.origin },
    { label: "Команда", value: s.command },
    { label: "Аргументы", value: s.args },
    { label: "URL", value: s.url },
  ];
}

function asRecord(v: unknown): Record<string, unknown> | undefined {
  return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : undefined;
}

/** Fields from a sub-object of the (already masked) server `config`, e.g. `env` / `headers`. */
function recordFields(prefix: string, s: McpServerEntry, key: string): Field[] {
  const rec = asRecord(asRecord(s.config)?.[key]);
  return Object.entries(maskRecord(rec)).map(([k, v]) => ({ label: `${prefix}.${k}`, value: v }));
}

const emptyDescription = computed(() => {
  const parts = [".mcp.json"];
  if (prefs.prefs.trackClaude) parts.push("~/.claude.json");
  if (prefs.prefs.trackCursor) parts.push("~/.cursor/mcp.json");
  parts.push("файлах настроек");
  const head = parts.slice(0, -1).join(", ");
  const last = parts[parts.length - 1];
  return `Серверы объявляются в ${head} и ${last}.`;
});
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    collapsed-content
    content-label="Исходный файл"
    empty-title="MCP-серверы не настроены"
    :empty-description="emptyDescription"
  >
    <template #detail="{ item }">
      <template v-if="byId.get(item.id)">
        <FieldList title="Подключение" :fields="connectionFields(byId.get(item.id)!)" />
        <FieldList
          v-if="recordFields('env', byId.get(item.id)!, 'env').length"
          title="Окружение"
          :fields="recordFields('env', byId.get(item.id)!, 'env')"
        />
        <FieldList
          v-if="recordFields('header', byId.get(item.id)!, 'headers').length"
          title="Заголовки"
          :fields="recordFields('header', byId.get(item.id)!, 'headers')"
        />
        <p class="flex items-center gap-1.5 text-2xs text-subtle">
          <Badge tone="neutral">скрыто</Badge> Секретные значения маскируются перед показом.
        </p>
      </template>
    </template>
  </ArtifactBrowser>
</template>
