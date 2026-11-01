<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import { PRODUCT_DIR_LABEL } from "@/lib/labels";
import { trackedProviders } from "@/lib/tooling";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const { t } = useI18n();
const scan = useScanStore();
const prefs = usePreferencesStore();

// Light inventory only: memory files, CLAUDE.md, workflows and output styles.
const items = computed(() =>
  scan
    .artifactsOfKind("memory", "claudeMd", "workflow", "outputStyle")
    .map(artifactToItem),
);

const emptyDescription = computed(() => {
  const dirs = trackedProviders(prefs.prefs).map((p) => PRODUCT_DIR_LABEL[p.id]);
  const listed =
    dirs.length === 0
      ? t("memory.emptyDirsFallback")
      : dirs.length === 1
        ? dirs[0]!
        : `${dirs.slice(0, -1).join(", ")} ${t("common.and")} ${dirs[dirs.length - 1]}`;
  return t("memory.emptyHint", { dirs: listed });
});
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    kind-filter
    :empty-title="t('empty.noMemory')"
    :empty-description="emptyDescription"
  />
</template>
