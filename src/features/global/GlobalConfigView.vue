<script setup lang="ts">
import { computed } from "vue";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import { isProjectScope } from "@/lib/normalize";
import Icon from "@/components/common/Icon.vue";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const scan = useScanStore();

// Global inventory: everything outside a project (user-level + managed + plugin).
const items = computed(() =>
  scan.artifacts
    .filter((a) => !isProjectScope(a.scope))
    .map(artifactToItem),
);

const hasManaged = computed(() => items.value.some((i) => i.scope === "managed"));
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div
      v-if="hasManaged"
      class="flex shrink-0 items-center gap-2 border-b border-line bg-elevated/40 px-4 py-1.5 text-xs text-muted"
    >
      <Icon name="lock" :size="12" />
      Управляемые настройки задаются организацией и показаны только для справки.
    </div>
    <div class="min-h-0 flex-1">
      <ArtifactBrowser
        :items="items"
        :loading="!scan.initialized"
        kind-filter
        show-kind
        empty-title="Глобальная конфигурация не найдена"
        empty-description="Ничего не обнаружено в ~/.claude или управляемых каталогах. Запустите сканирование для обновления."
      />
    </div>
  </div>
</template>
