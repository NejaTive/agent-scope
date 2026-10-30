<script setup lang="ts">
import { computed } from "vue";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const scan = useScanStore();

// Light inventory only: memory files, CLAUDE.md, workflows and output styles.
const items = computed(() =>
  scan
    .artifactsOfKind("memory", "claudeMd", "workflow", "outputStyle")
    .map(artifactToItem),
);
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    kind-filter
    empty-title="Файлы памяти не найдены"
    empty-description="Файлы CLAUDE.md, AGENTS.md, auto-memory, workflows и output styles из .claude и .cursor отображаются здесь."
  />
</template>
