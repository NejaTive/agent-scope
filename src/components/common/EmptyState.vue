<script setup lang="ts">
import Icon, { type IconName } from "./Icon.vue";

withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: IconName;
    tone?: "neutral" | "danger";
    compact?: boolean;
  }>(),
  { icon: "inbox", tone: "neutral", compact: false },
);
</script>

<template>
  <div
    class="flex h-full w-full flex-col items-center justify-center text-center"
    :class="compact ? 'gap-1 p-4' : 'gap-2 p-8'"
  >
    <div
      class="flex h-9 w-9 items-center justify-center rounded-lg border border-line bg-elevated"
      :class="tone === 'danger' ? 'text-danger' : 'text-subtle'"
    >
      <Icon :name="tone === 'danger' ? 'alert' : icon" :size="18" />
    </div>
    <div class="text-[13px] font-medium text-fg">{{ title }}</div>
    <p v-if="description" class="max-w-sm text-xs text-muted">{{ description }}</p>
    <div v-if="$slots.default" class="mt-2 flex items-center gap-2"><slot /></div>
  </div>
</template>
