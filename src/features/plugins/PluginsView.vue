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
    empty-title="Плагины не найдены"
    empty-description="Установленные плагины и записи enabledPlugins появляются здесь после сканирования."
  />
</template>
