<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { Scope } from "@/types/config";
import Badge, { type Tone } from "./Badge.vue";

const props = defineProps<{ scope: Scope }>();
const { t } = useI18n();

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
const label = computed(() => t(`labels.scope.${props.scope}` as `labels.scope.${Scope}`));
</script>

<template>
  <Badge :tone="tone">{{ label }}</Badge>
</template>
