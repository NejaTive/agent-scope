<script setup lang="ts">
import { computed } from "vue";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import { PRODUCT_DIR_LABEL } from "@/lib/labels";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const scan = useScanStore();
const prefs = usePreferencesStore();
const byId = computed(() => new Map(scan.rules.map((r) => [r.id, r])));

const items = computed<BrowserItem[]>(() =>
  scan.rules.map((r) => ({
    id: r.id,
    name: r.name,
    subtitle: r.paths.length ? `Применяется к ${r.paths.join(", ")}` : null,
    scope: r.scope,
    status: r.status,
    mtime: r.mtime,
    path: r.path,
    projectPath: r.projectPath,
    artifactId: r.artifactId,
    kind: "rule",
    product: r.product,
    searchText: (r.paths ?? []).join(" "),
  })),
);

const emptyDescription = computed(() => {
  const parts: string[] = [];
  if (prefs.prefs.trackClaude) parts.push(`${PRODUCT_DIR_LABEL.claude}/rules/*.md`);
  if (prefs.prefs.trackCursor) parts.push(`${PRODUCT_DIR_LABEL.cursor}/rules/*.mdc`);
  const where = parts.length ? parts.join(" и ") : "каталогах правил";
  return `Правила лежат в ${where} на уровне пользователя или проекта.`;
});
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    empty-title="Правила не найдены"
    :empty-description="emptyDescription"
  >
    <template #detail="{ item }">
      <FieldList
        title="Область"
        :fields="[
          { label: 'Глобы путей', value: byId.get(item.id)?.paths },
          { label: 'Условное', value: byId.get(item.id)?.conditional ? 'да (только для совпадающих путей)' : null },
        ]"
      />
    </template>
  </ArtifactBrowser>
</template>
