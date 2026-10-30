<script setup lang="ts">
import { computed } from "vue";
import type { Scope } from "@/types/config";
import { SCOPE_LABEL } from "@/lib/labels";
import Badge, { type Tone } from "./Badge.vue";

const props = defineProps<{ scope: Scope }>();

const TONE: Record<Scope, Tone> = {
  managed: "danger",
  user: "accent",
  project: "info",
  projectLocal: "warn",
  plugin: "neutral",
  nested: "info",
  globalConfig: "neutral",
};

const tone = computed(() => TONE[props.scope] ?? "neutral");
const label = computed(() => SCOPE_LABEL[props.scope] ?? props.scope);
</script>

<template>
  <Badge :tone="tone">{{ label }}</Badge>
</template>
