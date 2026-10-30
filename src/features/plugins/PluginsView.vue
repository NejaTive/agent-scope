<script setup lang="ts">
import { computed } from "vue";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const scan = useScanStore();
const items = computed(() => scan.plugins.map(artifactToItem));
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    empty-title="No plugins found"
    empty-description="Installed plugins and enabledPlugins entries appear here after a scan."
  />
</template>
