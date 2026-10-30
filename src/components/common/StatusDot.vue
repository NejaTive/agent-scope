<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    tone?: "ok" | "warn" | "danger" | "neutral" | "accent";
    pulse?: boolean;
    title?: string;
  }>(),
  { tone: "neutral", pulse: false },
);

const cls = computed(
  () =>
    ({
      ok: "bg-ok",
      warn: "bg-warn",
      danger: "bg-danger",
      accent: "bg-accent",
      neutral: "bg-subtle",
    })[props.tone],
);
</script>

<template>
  <span class="relative inline-flex h-2 w-2 shrink-0" :title="title">
    <span
      v-if="pulse"
      class="absolute inline-flex h-full w-full animate-ping rounded-full opacity-60"
      :class="cls"
    />
    <span class="relative inline-flex h-2 w-2 rounded-full" :class="cls" />
  </span>
</template>
