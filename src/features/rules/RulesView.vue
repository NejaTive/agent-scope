<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import { PRODUCT_DIR_LABEL } from "@/lib/labels";
import { trackedProviders } from "@/lib/tooling";
import type { Product } from "@/types/config";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const { t } = useI18n();
const scan = useScanStore();
const prefs = usePreferencesStore();
const byId = computed(() => new Map(scan.rules.map((r) => [r.id, r])));

const items = computed<BrowserItem[]>(() =>
  scan.rules.map((r) => ({
    id: r.id,
    name: r.name,
    subtitle: r.paths.length ? t("rules.appliesTo", { paths: r.paths.join(", ") }) : null,
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
  // Rule dirs are scanned for Claude Code (*.md) and Cursor (*.mdc) only.
  const RULE_GLOB: Partial<Record<Product, string>> = { claude: "*.md", cursor: "*.mdc" };
  const parts = trackedProviders(prefs.prefs)
    .filter((p) => RULE_GLOB[p.id])
    .map((p) => `${PRODUCT_DIR_LABEL[p.id]}/rules/${RULE_GLOB[p.id]}`);
  const where =
    parts.length === 0
      ? t("rules.emptyWhereFallback")
      : parts.length === 1
        ? parts[0]!
        : `${parts.slice(0, -1).join(", ")} ${t("common.and")} ${parts[parts.length - 1]}`;
  return t("rules.emptyHint", { where });
});

function detailFields(id: string) {
  const rule = byId.value.get(id);
  return [
    { label: t("rules.pathGlobs"), value: rule?.paths },
    {
      label: t("rules.conditional"),
      value: rule?.conditional ? t("rules.conditionalYes") : null,
    },
  ];
}
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    :empty-title="t('empty.noRules')"
    :empty-description="emptyDescription"
  >
    <template #detail="{ item }">
      <FieldList :title="t('rules.scope')" :fields="detailFields(item.id)" />
    </template>
  </ArtifactBrowser>
</template>
