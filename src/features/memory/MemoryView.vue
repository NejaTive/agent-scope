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
    empty-title="No memory files found"
    empty-description="CLAUDE.md files, auto-memory, workflows and output styles discovered in scanned scopes show up here."
  />
</template>
