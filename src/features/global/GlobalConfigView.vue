<script setup lang="ts">
import { computed } from "vue";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import { shortenPath } from "@/lib/format";
import { PRODUCT_LABEL, resolveProduct } from "@/lib/labels";
import { isProjectScope } from "@/lib/normalize";
import Badge from "@/components/common/Badge.vue";
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

const claudeRoot = computed(() => scan.globalMeta.configDir || "~/.claude");
const cursorRoot = computed(
  () => scan.globalMeta.cursorConfigDir || "~/.cursor",
);

const productCounts = computed(() => {
  let claude = 0;
  let cursor = 0;
  for (const i of items.value) {
    if (resolveProduct(i.product) === "cursor") cursor += 1;
    else claude += 1;
  }
  return { claude, cursor };
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div
      class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-line bg-elevated/30 px-4 py-1.5 text-xs text-muted"
    >
      <span class="flex items-center gap-1.5">
        <Badge tone="info">{{ PRODUCT_LABEL.claude }}</Badge>
        <span class="font-mono text-2xs" :title="claudeRoot">{{ shortenPath(claudeRoot) }}</span>
        <span class="font-mono text-2xs text-subtle">{{ productCounts.claude }}</span>
      </span>
      <span class="text-subtle">·</span>
      <span class="flex items-center gap-1.5">
        <Badge tone="accent">{{ PRODUCT_LABEL.cursor }}</Badge>
        <span class="font-mono text-2xs" :title="cursorRoot">{{ shortenPath(cursorRoot) }}</span>
        <span class="font-mono text-2xs text-subtle">{{ productCounts.cursor }}</span>
      </span>
      <span
        v-if="hasManaged"
        class="ml-auto flex items-center gap-1.5 text-xs text-muted"
      >
        <Icon name="lock" :size="12" />
        Управляемые настройки задаются организацией и показаны только для справки.
      </span>
    </div>
    <div class="min-h-0 flex-1">
      <ArtifactBrowser
        :items="items"
        :loading="!scan.initialized"
        kind-filter
        show-kind
        empty-title="Глобальные не найдены"
        empty-description="Ничего не обнаружено в ~/.claude, ~/.cursor или управляемых каталогах. Запустите сканирование для обновления."
      />
    </div>
  </div>
</template>
