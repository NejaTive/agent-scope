<script setup lang="ts">
import { computed } from "vue";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const scan = useScanStore();
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
    searchText: (r.paths ?? []).join(" "),
  })),
);
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    empty-title="Правила не найдены"
    empty-description="Правила лежат в .claude/rules/*.md на уровне пользователя или проекта."
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
