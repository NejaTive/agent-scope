<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useScanStore } from "@/stores/scan";
import { artifactToItem } from "@/lib/browser";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";

const { t } = useI18n();
const scan = useScanStore();
const items = computed(() => scan.plugins.map(artifactToItem));
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    :empty-title="t('empty.noPlugins')"
    :empty-description="t('plugins.emptyHint')"
  />
</template>
