<script setup lang="ts">
import { computed } from "vue";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const scan = useScanStore();
const byId = computed(() => new Map(scan.hooks.map((h) => [h.id, h])));

const items = computed<BrowserItem[]>(() =>
  scan.hooks.map((h) => ({
    id: h.id,
    name: h.matcher ? `${h.event} · ${h.matcher}` : h.event,
    subtitle: h.command ?? h.prompt ?? h.hookType,
    scope: h.scope,
    status: h.status,
    mtime: h.mtime,
    path: h.sourcePath,
    projectPath: h.projectPath,
    artifactId: h.sourcePath,
    kind: "hook",
    searchText: [h.event, h.matcher, h.hookType, h.command, h.prompt].filter(Boolean).join(" "),
  })),
);
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    collapsed-content
    content-label="Исходный файл"
    empty-title="Хуки не настроены"
    empty-description="Хуки объявляются в settings.json (пользователь, проект или локальный) и в плагинах."
  >
    <template #detail="{ item }">
      <FieldList
        v-if="byId.get(item.id)"
        title="Хук"
        :fields="[
          { label: 'Событие', value: byId.get(item.id)?.event },
          { label: 'Матчер', value: byId.get(item.id)?.matcher },
          { label: 'Тип', value: byId.get(item.id)?.hookType },
          { label: 'Команда', value: byId.get(item.id)?.command },
          { label: 'Таймаут', value: byId.get(item.id)?.timeout != null ? `${byId.get(item.id)?.timeout}s` : null },
          { label: 'Промпт', value: byId.get(item.id)?.prompt },
          { label: 'Объявлен в', value: byId.get(item.id)?.sourcePath },
        ]"
      />
    </template>
  </ArtifactBrowser>
</template>
