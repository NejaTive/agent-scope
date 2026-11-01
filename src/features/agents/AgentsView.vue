<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import { PRODUCT_DIR_LABEL } from "@/lib/labels";
import { trackedProviders } from "@/lib/tooling";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const { t } = useI18n();
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

const emptyDescription = computed(() => {
  // Subagent dirs are scanned for Claude Code and Cursor only (no Codex equivalent).
  const parts = trackedProviders(prefs.prefs)
    .filter((p) => p.id !== "codex")
    .map((p) => `${PRODUCT_DIR_LABEL[p.id]}/agents/*.md`);
  const where =
    parts.length === 0
      ? "agents/*.md"
      : parts.length === 1
        ? parts[0]!
        : `${parts.slice(0, -1).join(", ")} ${t("common.and")} ${parts[parts.length - 1]}`;
  return t("agents.emptyHint", { where });
});

function detailFields(id: string) {
  const agent = byId.value.get(id);
  return [
    { label: t("agents.model"), value: agent?.model },
    { label: t("agents.tools"), value: agent?.tools },
    {
      label: t("agents.validity"),
      value: agent?.valid === false ? t("agents.invalid") : null,
    },
  ];
}
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    :empty-title="t('empty.noAgents')"
    :empty-description="emptyDescription"
  >
    <template #detail="{ item }">
      <FieldList :title="t('agents.frontmatter')" :fields="detailFields(item.id)" />
    </template>
  </ArtifactBrowser>
</template>
