<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import { trackedDirLabels } from "@/lib/tooling";
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

const emptyDescription = computed(() =>
  t("memory.emptyHint", { dirs: trackedDirLabels(prefs.prefs) }),
);
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
