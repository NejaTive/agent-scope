<script setup lang="ts">
import { computed } from "vue";
import type { Product } from "@/types/config";
import { PRODUCT_LABEL, resolveProduct } from "@/lib/labels";
import Badge, { type Tone } from "./Badge.vue";

const props = defineProps<{ product?: Product | null }>();

const TONE: Record<Product, Tone> = {
  claude: "info",
  cursor: "accent",
  codex: "ok",
};

const resolved = computed(() => resolveProduct(props.product));
const tone = computed(() => TONE[resolved.value]);
const label = computed(() => PRODUCT_LABEL[resolved.value]);
</script>

<template>
  <Badge :tone="tone" :title="label">{{ label }}</Badge>
</template>
