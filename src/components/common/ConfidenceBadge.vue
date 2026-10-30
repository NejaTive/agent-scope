<script setup lang="ts">
import { computed } from "vue";
import type { Confidence } from "@/types/config";
import { CONFIDENCE_LABEL } from "@/lib/labels";
import Badge, { type Tone } from "./Badge.vue";
import StatusDot from "./StatusDot.vue";

const props = defineProps<{ confidence: Confidence }>();

const TONE: Record<Confidence, Tone> = {
  computed: "ok",
  inferred: "info",
  unknown: "neutral",
};

const tone = computed(() => TONE[props.confidence] ?? "neutral");
const dot = computed(() => (tone.value === "info" ? "accent" : tone.value) as "ok" | "warn" | "neutral" | "accent");
</script>

<template>
  <Badge :tone="tone" :title="`Уверенность: ${confidence}`">
    <StatusDot :tone="dot" />
    {{ CONFIDENCE_LABEL[confidence] ?? confidence }}
  </Badge>
</template>
