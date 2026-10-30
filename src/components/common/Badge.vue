<script setup lang="ts">
import { computed } from "vue";

export type Tone = "neutral" | "accent" | "ok" | "warn" | "danger" | "info";

const props = withDefaults(
  defineProps<{ tone?: Tone; mono?: boolean; title?: string }>(),
  { tone: "neutral", mono: false },
);

const toneClass = computed(
  () =>
    ({
      neutral: "bg-elevated text-muted border-line",
      accent: "bg-accent/10 text-accent border-accent/30",
      ok: "bg-ok/10 text-ok border-ok/30",
      warn: "bg-warn/10 text-warn border-warn/30",
      danger: "bg-danger/10 text-danger border-danger/30",
      info: "bg-info/10 text-info border-info/30",
    })[props.tone],
);
</script>

<template>
  <span
    :title="title"
    class="inline-flex h-[18px] shrink-0 items-center gap-1 whitespace-nowrap rounded border px-1.5 text-2xs font-medium leading-none"
    :class="[toneClass, mono && 'font-mono']"
  >
    <slot />
  </span>
</template>
