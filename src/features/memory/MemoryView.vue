<script setup lang="ts">
import { computed } from "vue";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import { trackedDirLabels } from "@/lib/tooling";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const scan = useScanStore();
const prefs = usePreferencesStore();

// Light inventory only: memory files, CLAUDE.md, workflows and output styles.
const items = computed(() =>
  scan
    .artifactsOfKind("memory", "claudeMd", "workflow", "outputStyle")
    .map(artifactToItem),
);

const emptyDescription = computed(
  () =>
    `Файлы CLAUDE.md, AGENTS.md, auto-memory, workflows и output styles из ${trackedDirLabels(prefs.prefs).replace(/ \/ /g, " и ")} отображаются здесь.`,
);
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    kind-filter
    empty-title="Файлы памяти не найдены"
    :empty-description="emptyDescription"
  />
</template>
