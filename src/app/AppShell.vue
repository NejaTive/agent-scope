<script setup lang="ts">
import { onBeforeUnmount, onMounted, watch } from "vue";
import { RouterView, useRoute } from "vue-router";
import { useScanStore } from "@/stores/scan";
import { usePreferencesStore } from "@/stores/preferences";
import { useUiStore } from "@/stores/ui";
import Sidebar from "@/components/layout/Sidebar.vue";
import AppHeader from "@/components/layout/AppHeader.vue";
import Icon from "@/components/common/Icon.vue";

const scan = useScanStore();
const prefs = usePreferencesStore();
const ui = useUiStore();
const route = useRoute();

// Clear the header search when switching sections.
watch(
  () => route.path,
  () => ui.clearSearch(),
);

onMounted(async () => {
  await prefs.load();
  await scan.listenForChanges();
  // Toggling the preference later starts/stops the watcher from the preferences store.
  if (prefs.prefs.watchOnStart) void prefs.applyWatching(true);
  await scan.init(true);
});

onBeforeUnmount(() => scan.stopListening());
</script>

<template>
  <div class="flex h-full w-full overflow-hidden bg-bg text-fg">
    <Sidebar />
    <div class="flex min-w-0 flex-1 flex-col">
      <AppHeader />
      <div
        v-if="scan.error"
        class="flex shrink-0 items-start gap-2 border-b border-warn/30 bg-warn/10 px-4 py-1.5 text-xs text-warn"
      >
        <Icon name="alert" :size="14" class="mt-px" />
        <span class="min-w-0 flex-1 break-words">
          {{ scan.backendAvailable ? "Ошибка сканирования" : "Бэкенд недоступен" }}: {{ scan.error }}
        </span>
      </div>
      <main class="min-h-0 flex-1 overflow-hidden">
        <RouterView v-slot="{ Component, route: r }">
          <component :is="Component" :key="r.name === 'project-detail' ? r.fullPath : r.name" />
        </RouterView>
      </main>
    </div>
  </div>
</template>
