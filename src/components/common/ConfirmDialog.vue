<script setup lang="ts">
import { onMounted, onUnmounted, watch } from "vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
  }>(),
  {
    description: "",
    confirmLabel: "Подтвердить",
    cancelLabel: "Отмена",
    danger: false,
  },
);

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();

function onKey(e: KeyboardEvent) {
  if (!props.open) return;
  if (e.key === "Escape") emit("cancel");
}

onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
  document.body.style.overflow = "";
});

watch(
  () => props.open,
  (v) => {
    document.body.style.overflow = v ? "hidden" : "";
  },
);
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="presentation"
      @click.self="emit('cancel')"
    >
      <div
        role="dialog"
        aria-modal="true"
        class="card w-full max-w-md space-y-3 px-4 py-3 shadow-lg"
        @click.stop
      >
        <h3 class="text-sm font-semibold">{{ title }}</h3>
        <p v-if="description" class="text-xs text-muted whitespace-pre-wrap">{{ description }}</p>
        <slot />
        <div class="flex justify-end gap-2 pt-1">
          <button type="button" class="btn !h-7" @click="emit('cancel')">
            {{ cancelLabel }}
          </button>
          <button
            type="button"
            class="btn !h-7"
            :class="danger ? 'text-danger' : 'btn-accent'"
            @click="emit('confirm')"
          >
            {{ confirmLabel }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
