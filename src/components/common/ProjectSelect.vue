<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";

withDefaults(defineProps<{ allLabel?: string }>(), { allLabel: "Все проекты" });

const scan = useScanStore();
const { selectedProjectPath } = storeToRefs(useUiStore());

function onChange(e: Event) {
  const v = (e.target as HTMLSelectElement).value;
  selectedProjectPath.value = v || null;
}
</script>

<template>
  <label class="flex items-center gap-1.5 text-xs text-muted">
    <span>Проект</span>
    <select
      class="input max-w-[260px] pr-6"
      :value="selectedProjectPath ?? ''"
      @change="onChange"
    >
      <option value="">{{ allLabel }}</option>
      <option v-for="p in scan.projects" :key="p.path" :value="p.path">{{ p.name }}</option>
    </select>
  </label>
</template>
