<script setup lang="ts">
import { computed, ref } from "vue";
import { highlight } from "@/lib/highlight";
import Icon from "@/components/common/Icon.vue";

const props = withDefaults(
  defineProps<{
    code: string;
    language?: string | null;
    maxHeight?: string;
    lineNumbers?: boolean;
    copyable?: boolean;
  }>(),
  { language: null, lineNumbers: false, copyable: true },
);

const html = computed(() => highlight(props.code, props.language));
const lineCount = computed(() => props.code.split("\n").length);
const copied = ref(false);

async function copy() {
  try {
    await navigator.clipboard.writeText(props.code);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1200);
  } catch {
    /* clipboard unavailable */
  }
}
</script>

<template>
  <div class="group relative overflow-hidden rounded-md border border-line bg-elevated/60">
    <button
      v-if="copyable && code"
      type="button"
      class="btn-ghost absolute right-1.5 top-1.5 z-10 !h-6 bg-surface/80 opacity-0 backdrop-blur transition-opacity group-hover:opacity-100"
      @click="copy"
    >
      <Icon name="copy" :size="12" />
      {{ copied ? "Скопировано" : "Копировать" }}
    </button>
    <div class="flex overflow-auto" :style="maxHeight ? { maxHeight } : undefined">
      <pre
        v-if="lineNumbers"
        class="select-none border-r border-line px-2 py-2.5 text-right font-mono text-[11.5px] leading-[1.6] text-subtle"
        aria-hidden="true"
      >{{ Array.from({ length: lineCount }, (_, i) => i + 1).join("\n") }}</pre>
      <pre class="min-w-0 flex-1 px-3 py-2.5 font-mono text-[11.5px] leading-[1.6]"><code class="hljs" v-html="html" /></pre>
    </div>
  </div>
</template>
