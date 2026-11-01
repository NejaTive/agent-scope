<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

const props = withDefaults(defineProps<{ label?: string; lines?: number }>(), {
  label: undefined,
  lines: 0,
});

const { t } = useI18n();
const resolvedLabel = computed(() => props.label ?? t("common.loading"));
</script>

<template>
  <div v-if="lines > 0" class="space-y-2 p-4" role="status" :aria-label="resolvedLabel">
    <div
      v-for="i in lines"
      :key="i"
      class="h-3 animate-pulse rounded bg-elevated"
      :style="{ width: `${90 - ((i * 17) % 45)}%` }"
    />
  </div>
  <div v-else class="flex items-center gap-2 p-4 text-xs text-muted" role="status">
    <span class="h-3 w-3 animate-spin rounded-full border border-subtle border-t-accent" />
    {{ resolvedLabel }}
  </div>
</template>
