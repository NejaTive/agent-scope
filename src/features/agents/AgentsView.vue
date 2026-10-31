<script setup lang="ts">
import { computed } from "vue";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import { trackedPathHint } from "@/lib/tooling";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const scan = useScanStore();
const prefs = usePreferencesStore();
const byId = computed(() => new Map(scan.agents.map((a) => [a.id, a])));

const items = computed<BrowserItem[]>(() =>
  scan.agents.map((a) => ({
    id: a.id,
    name: a.name,
    subtitle: a.description ?? null,
    scope: a.scope,
    status: a.status,
    mtime: a.mtime,
    path: a.path,
    projectPath: a.projectPath,
    artifactId: a.artifactId,
    kind: "agent",
    product: a.product,
    searchText: [a.model, ...(a.tools ?? [])].filter(Boolean).join(" "),
  })),
);

const emptyDescription = computed(
  () =>
    `Субагенты лежат в ${trackedPathHint(prefs.prefs, "agents/*.md")} на уровне пользователя или проекта.`,
);
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    empty-title="Агенты не найдены"
    :empty-description="emptyDescription"
  >
    <template #detail="{ item }">
      <FieldList
        title="Frontmatter"
        :fields="[
          { label: 'Модель', value: byId.get(item.id)?.model },
          { label: 'Инструменты', value: byId.get(item.id)?.tools },
          { label: 'Валидность', value: byId.get(item.id)?.valid === false ? 'нет имени или описания' : null },
        ]"
      />
    </template>
  </ArtifactBrowser>
</template>
