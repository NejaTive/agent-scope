<script setup lang="ts">
import Icon from "./Icon.vue";

defineProps<{ modelValue: string; placeholder?: string }>();
const emit = defineEmits<{
  "update:modelValue": [value: string];
  focus: [];
  blur: [];
  enter: [];
}>();

function onInput(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="relative">
    <Icon name="search" :size="13" class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-subtle" />
    <input
      :value="modelValue"
      type="text"
      spellcheck="false"
      autocomplete="off"
      class="input w-full pl-7 pr-6"
      :placeholder="placeholder ?? 'Поиск…'"
      @input="onInput"
      @focus="emit('focus')"
      @blur="emit('blur')"
      @keydown.enter="emit('enter')"
      @keydown.esc="emit('update:modelValue', '')"
    />
    <button
      v-if="modelValue"
      type="button"
      class="absolute right-1.5 top-1/2 -translate-y-1/2 text-subtle hover:text-fg"
      title="Очистить"
      @mousedown.prevent
      @click="emit('update:modelValue', '')"
    >
      <Icon name="x" :size="12" />
    </button>
  </div>
</template>
