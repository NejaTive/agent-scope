<script setup lang="ts">
import { computed } from "vue";
import { maskDeep } from "@/lib/secrets";
import CodeViewer from "./CodeViewer.vue";

const props = withDefaults(
  defineProps<{
    /** Either a parsed value… */
    value?: unknown;
    /** …or raw JSON text. */
    text?: string;
    maxHeight?: string;
    lineNumbers?: boolean;
  }>(),
  { maxHeight: undefined, lineNumbers: false },
);

const parsed = computed<{ code: string; error: string | null }>(() => {
  if (props.text !== undefined) {
    try {
      const v = maskDeep(JSON.parse(props.text));
      return { code: JSON.stringify(v, null, 2), error: null };
    } catch (e) {
      return { code: props.text, error: e instanceof Error ? e.message : "Некорректный JSON" };
    }
  }
  try {
    return { code: JSON.stringify(maskDeep(props.value), null, 2) ?? "null", error: null };
  } catch {
    return { code: String(props.value), error: null };
  }
});
</script>

<template>
  <div class="space-y-1.5">
    <div
      v-if="parsed.error"
      class="rounded-md border border-warn/30 bg-warn/10 px-2 py-1 text-xs text-warn"
    >
      Некорректный JSON: {{ parsed.error }}
    </div>
    <CodeViewer
      :code="parsed.code"
      language="json"
      :max-height="maxHeight"
      :line-numbers="lineNumbers"
    />
  </div>
</template>
