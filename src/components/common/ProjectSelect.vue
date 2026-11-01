<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { storeToRefs } from "pinia";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";

const props = withDefaults(defineProps<{ allLabel?: string; block?: boolean }>(), {
  allLabel: undefined,
  block: false,
});

const { t } = useI18n();
const resolvedAllLabel = computed(() => props.allLabel ?? t("projectSelect.all"));

const scan = useScanStore();
const { selectedProjectPath } = storeToRefs(useUiStore());

function onChange(e: Event) {
  const v = (e.target as HTMLSelectElement).value;
  selectedProjectPath.value = v || null;
}
</script>

<template>
  <label
    class="flex min-w-0 text-xs text-muted"
    :class="block ? 'w-full flex-col items-stretch gap-1' : 'max-w-[280px] items-center gap-1.5'"
  >
    <span class="shrink-0">{{ t("projectSelect.label") }}</span>
    <select
      class="input min-w-0 pr-6"
      :class="block ? 'w-full max-w-full' : 'w-0 flex-1'"
      :value="selectedProjectPath ?? ''"
      @change="onChange"
    >
      <option value="">{{ resolvedAllLabel }}</option>
      <option v-for="p in scan.projects" :key="p.path" :value="p.path">{{ p.name }}</option>
    </select>
  </label>
</template>
