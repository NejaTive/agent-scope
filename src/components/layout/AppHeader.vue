<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import { useTheme } from "@/composables/useTheme";
import { ipc } from "@/lib/ipc";
import { KIND_LABEL, routeForKind } from "@/lib/labels";
import { shortenPath } from "@/lib/format";
import type { SearchHit } from "@/types/config";
import Icon from "@/components/common/Icon.vue";
import SearchInput from "@/components/common/SearchInput.vue";
import ScopeBadge from "@/components/common/ScopeBadge.vue";
import Badge from "@/components/common/Badge.vue";

const route = useRoute();
const router = useRouter();
const scan = useScanStore();
const ui = useUiStore();
const { search } = storeToRefs(ui);
const theme = useTheme();

const title = computed(() => (route.meta.title as string | undefined) ?? "Agent Scope");
const searchable = computed(() => !!route.meta.searchable);

const themeIcon = computed(() =>
  theme.preference.value === "dark" ? "moon" : theme.preference.value === "light" ? "sun" : "monitor",
);
const themeLabel = computed(() => `Theme: ${theme.preference.value} (click to change)`);

const changeCount = computed(() => scan.recentChanges.length);

function openChanges() {
  ui.markChangesSeen();
  void router.push("/");
}

// ---- global search dropdown (backend search_artifacts) ---------------------
const focused = ref(false);
const hits = ref<SearchHit[]>([]);
const searching = ref(false);
let timer: ReturnType<typeof setTimeout> | null = null;
let token = 0;

watch(search, (q) => {
  if (timer) clearTimeout(timer);
  const query = q.trim();
  if (query.length < 2) {
    hits.value = [];
    searching.value = false;
    return;
  }
  searching.value = true;
  timer = setTimeout(async () => {
    const my = ++token;
    try {
      const res = (await ipc.searchArtifacts(query)).slice(0, 8);
      if (my === token) hits.value = Array.isArray(res) ? res : [];
    } catch {
      if (my === token) hits.value = [];
    } finally {
      if (my === token) searching.value = false;
    }
  }, 220);
});

const showDropdown = computed(
  () => focused.value && search.value.trim().length >= 2 && (hits.value.length > 0 || searching.value),
);

function openHit(hit: SearchHit) {
  ui.focusArtifactId = hit.path;
  ui.search = "";
  hits.value = [];
  void router.push(routeForKind(hit.kind));
}
</script>

<template>
  <header
    class="flex h-11 shrink-0 items-center gap-3 border-b border-line bg-surface px-4"
    data-tauri-drag-region
  >
    <h1 class="min-w-0 flex-1 truncate text-[13px] font-semibold tracking-tight">
      {{ title }}
    </h1>

    <div class="relative w-72">
      <SearchInput
        v-model="search"
        :placeholder="searchable ? 'Filter this view · search all…' : 'Search all artifacts…'"
        @focus="focused = true"
        @blur="focused = false"
      />
      <div
        v-if="showDropdown"
        class="absolute left-0 right-0 top-8 z-30 overflow-hidden rounded-md border border-line bg-surface shadow-lg shadow-black/30"
      >
        <div v-if="searching && !hits.length" class="px-3 py-2 text-xs text-muted">Searching…</div>
        <button
          v-for="hit in hits"
          :key="hit.id"
          type="button"
          class="flex w-full flex-col gap-0.5 px-3 py-1.5 text-left hover:bg-elevated"
          @mousedown.prevent="openHit(hit)"
        >
          <span class="flex items-center gap-1.5">
            <span class="truncate text-xs font-medium">{{ hit.name }}</span>
            <Badge>{{ KIND_LABEL[hit.kind] ?? hit.kind }}</Badge>
            <ScopeBadge :scope="hit.scope" />
          </span>
          <span class="truncate font-mono text-2xs text-subtle">{{ shortenPath(hit.path) }}</span>
        </button>
      </div>
    </div>

    <div class="flex items-center gap-1">
      <button
        type="button"
        class="btn-ghost relative"
        :title="
          ui.unseenChanges
            ? `${ui.unseenChanges} new change(s) detected`
            : `${changeCount} recent change(s)`
        "
        @click="openChanges"
      >
        <Icon name="activity" :size="14" />
        <span class="font-mono tabular-nums">{{ changeCount }}</span>
        <span
          v-if="ui.unseenChanges"
          class="absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-accent"
        />
      </button>

      <button
        type="button"
        class="btn"
        :disabled="scan.scanning"
        title="Rescan configuration"
        @click="scan.refresh()"
      >
        <Icon name="refresh" :size="13" :class="scan.scanning && 'animate-spin'" />
        Refresh
      </button>

      <button type="button" class="btn-ghost !px-1.5" :title="themeLabel" @click="theme.cycle()">
        <Icon :name="themeIcon" :size="14" />
      </button>
    </div>
  </header>
</template>
