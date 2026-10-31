<script setup lang="ts">
import { computed } from "vue";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import { shortenPath } from "@/lib/format";
import { PRODUCT_LABEL, PRODUCT_DIR_LABEL, resolveProduct } from "@/lib/labels";
import { isProjectScope } from "@/lib/normalize";
import { isProductTracked } from "@/lib/tooling";
import Badge from "@/components/common/Badge.vue";
import Icon from "@/components/common/Icon.vue";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const scan = useScanStore();
const prefs = usePreferencesStore();

// Global inventory: everything outside a project (user-level + managed + plugin).
const items = computed(() =>
  scan.artifacts
    .filter((a) => !isProjectScope(a.scope))
    .filter((a) => isProductTracked(prefs.prefs, resolveProduct(a.product)))
    .map(artifactToItem),
);

const hasManaged = computed(() => items.value.some((i) => i.scope === "managed"));

const claudeRoot = computed(() => scan.globalMeta.configDir || "~/.claude");
const cursorRoot = computed(
  () => scan.globalMeta.cursorConfigDir || "~/.cursor",
);

const showClaude = computed(() => prefs.prefs.trackClaude);
const showCursor = computed(() => prefs.prefs.trackCursor);

const productCounts = computed(() => {
  let claude = 0;
  let cursor = 0;
  for (const i of items.value) {
    if (resolveProduct(i.product) === "cursor") cursor += 1;
    else claude += 1;
  }
  return { claude, cursor };
});

const emptyDescription = computed(() => {
  const roots: string[] = [];
  if (prefs.prefs.trackClaude) roots.push(`~/${PRODUCT_DIR_LABEL.claude}`);
  if (prefs.prefs.trackCursor) roots.push(`~/${PRODUCT_DIR_LABEL.cursor}`);
  const listed = roots.length ? roots.join(", ") : "каталогах конфигурации";
  return `Ничего не обнаружено в ${listed} или управляемых каталогах. Запустите сканирование для обновления.`;
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div
      v-if="showClaude || showCursor || hasManaged"
      class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-line bg-elevated/30 px-4 py-1.5 text-xs text-muted"
    >
      <span v-if="showClaude" class="flex items-center gap-1.5">
        <Badge tone="info">{{ PRODUCT_LABEL.claude }}</Badge>
        <span class="font-mono text-2xs" :title="claudeRoot">{{ shortenPath(claudeRoot) }}</span>
        <span class="font-mono text-2xs text-subtle">{{ productCounts.claude }}</span>
      </span>
      <span v-if="showClaude && showCursor" class="text-subtle">·</span>
      <span v-if="showCursor" class="flex items-center gap-1.5">
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
        :empty-description="emptyDescription"
      />
    </div>
  </div>
</template>
