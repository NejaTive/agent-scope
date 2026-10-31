<script setup lang="ts">
import { computed } from "vue";
import { formatValue } from "@/lib/format";

export interface Field {
  label: string;
  value: unknown;
  mono?: boolean;
}

const props = defineProps<{ title?: string; fields: Field[] }>();

function isEmpty(v: unknown) {
  return v === undefined || v === null || v === "" || (Array.isArray(v) && v.length === 0);
}

const visible = computed(() => props.fields.filter((f) => !isEmpty(f.value)));
</script>

<template>
  <section v-if="visible.length">
    <div v-if="title" class="label mb-1.5">{{ title }}</div>
    <dl class="card grid grid-cols-[max-content_1fr] gap-x-5 gap-y-1 px-3 py-2 text-xs">
      <template v-for="f in visible" :key="f.label">
        <dt class="text-subtle">{{ f.label }}</dt>
        <dd class="min-w-0 break-words" :class="f.mono !== false && 'font-mono text-[11.5px]'">
          <template v-if="Array.isArray(f.value)">
            <span v-for="(v, i) in f.value" :key="i" class="mr-1 inline-block rounded bg-elevated px-1 py-px">
              {{ formatValue(v) }}
            </span>
          </template>
          <template v-else>{{ formatValue(f.value) }}</template>
        </dd>
      </template>
    </dl>
  </section>
</template>
