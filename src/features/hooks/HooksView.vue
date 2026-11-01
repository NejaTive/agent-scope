<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const { t } = useI18n();
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
    product: h.product,
    searchText: [h.event, h.matcher, h.hookType, h.command, h.prompt].filter(Boolean).join(" "),
  })),
);

function detailFields(id: string) {
  const h = byId.value.get(id);
  return [
    { label: t("hooks.event"), value: h?.event },
    { label: t("hooks.matcher"), value: h?.matcher },
    { label: t("hooks.type"), value: h?.hookType },
    { label: t("hooks.command"), value: h?.command },
    { label: t("hooks.timeout"), value: h?.timeout != null ? `${h.timeout}s` : null },
    { label: t("hooks.prompt"), value: h?.prompt },
    { label: t("hooks.declaredIn"), value: h?.sourcePath },
  ];
}
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    collapsed-content
    :content-label="t('hooks.sourceFile')"
    :empty-title="t('hooks.emptyTitle')"
    :empty-description="t('hooks.emptyHint')"
  >
    <template #detail="{ item }">
      <FieldList
        v-if="byId.get(item.id)"
        :title="t('hooks.hook')"
        :fields="detailFields(item.id)"
      />
    </template>
  </ArtifactBrowser>
</template>
