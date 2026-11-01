<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import { shortenPath } from "@/lib/format";
import { PRODUCT_DIR_LABEL, resolveProduct } from "@/lib/labels";
import { isProjectScope } from "@/lib/normalize";
import type { Product } from "@/types/config";
import { isProductTracked, trackedProviders } from "@/lib/tooling";
import ProductBadge from "@/components/common/ProductBadge.vue";
import Icon from "@/components/common/Icon.vue";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const { t } = useI18n();
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

const rootByProduct = computed<Record<Product, string>>(() => ({
  claude: scan.globalMeta.configDir || `~/${PRODUCT_DIR_LABEL.claude}`,
  cursor: scan.globalMeta.cursorConfigDir || `~/${PRODUCT_DIR_LABEL.cursor}`,
  codex: scan.globalMeta.codexConfigDir || `~/${PRODUCT_DIR_LABEL.codex}`,
}));

const productRows = computed(() => {
  const counts: Record<string, number> = {};
  for (const i of items.value) {
    const product = resolveProduct(i.product);
    counts[product] = (counts[product] ?? 0) + 1;
  }
  return trackedProviders(prefs.prefs).map((prov) => ({
    id: prov.id,
    root: rootByProduct.value[prov.id],
    count: counts[prov.id] ?? 0,
  }));
});

const emptyDescription = computed(() => {
  const roots = trackedProviders(prefs.prefs).map((p) => `~/${PRODUCT_DIR_LABEL[p.id]}`);
  const listed = roots.length ? roots.join(", ") : t("global.emptyRootsFallback");
  return t("global.emptyDescription", { roots: listed });
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div
      v-if="productRows.length || hasManaged"
      class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-line bg-elevated/30 px-4 py-1.5 text-xs text-muted"
    >
      <template v-for="(row, i) in productRows" :key="row.id">
        <span v-if="i > 0" class="text-subtle">·</span>
        <span class="flex items-center gap-1.5">
          <ProductBadge :product="row.id" />
          <span class="font-mono text-2xs" :title="row.root">{{ shortenPath(row.root) }}</span>
          <span class="font-mono text-2xs text-subtle">{{ row.count }}</span>
        </span>
      </template>
      <span
        v-if="hasManaged"
        class="ml-auto flex items-center gap-1.5 text-xs text-muted"
      >
        <Icon name="lock" :size="12" />
        {{ t("global.managedHint") }}
      </span>
    </div>
    <div class="min-h-0 flex-1">
      <ArtifactBrowser
        :items="items"
        :loading="!scan.initialized"
        kind-filter
        show-kind
        :empty-title="t('global.emptyTitle')"
        :empty-description="emptyDescription"
      />
    </div>
  </div>
</template>
