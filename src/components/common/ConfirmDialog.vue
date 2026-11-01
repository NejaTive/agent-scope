<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from "vue";
import { useI18n } from "vue-i18n";

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
    confirmLabel: undefined,
    cancelLabel: undefined,
    danger: false,
  },
);

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();

const { t } = useI18n();
const confirmText = computed(() => props.confirmLabel ?? t("common.confirm"));
const cancelText = computed(() => props.cancelLabel ?? t("common.cancel"));

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
            {{ cancelText }}
          </button>
          <button
            type="button"
            class="btn !h-7"
            :class="danger ? 'text-danger' : 'btn-accent'"
            @click="emit('confirm')"
          >
            {{ confirmText }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
