<script setup lang="ts">
withDefaults(defineProps<{ label?: string; lines?: number }>(), {
  label: "Загрузка…",
  lines: 0,
});
</script>

<template>
  <div v-if="lines > 0" class="space-y-2 p-4" role="status" :aria-label="label">
    <div
      v-for="i in lines"
      :key="i"
      class="h-3 animate-pulse rounded bg-elevated"
      :style="{ width: `${90 - ((i * 17) % 45)}%` }"
    />
  </div>
  <div v-else class="flex items-center gap-2 p-4 text-xs text-muted" role="status">
    <span class="h-3 w-3 animate-spin rounded-full border border-subtle border-t-accent" />
    {{ label }}
  </div>
</template>
