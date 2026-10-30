<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { useSettingsDocs } from "@/composables/useSettingsDocs";
import { matchesQuery } from "@/composables/useSearch";
import { useUiStore } from "@/stores/ui";
import { useScanStore } from "@/stores/scan";
import {
  CATEGORY_LABEL,
  SETTINGS_CATEGORIES,
  categorizeSetting,
  type SettingsCategory,
} from "@/lib/settingsCategories";
import { SCOPE_PRECEDENCE } from "@/lib/labels";
import { maskIfSecret } from "@/lib/secrets";
import { relativeTime, shortenPath } from "@/lib/format";
import type { Scope } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Icon from "@/components/common/Icon.vue";
import LoadingBlock from "@/components/common/LoadingBlock.vue";
import ProjectSelect from "@/components/common/ProjectSelect.vue";
import ScopeBadge from "@/components/common/ScopeBadge.vue";
import JsonViewer from "@/components/viewers/JsonViewer.vue";
import SettingsDiffPanel from "./SettingsDiffPanel.vue";

interface SettingRow {
  id: string;
  key: string;
  value: unknown;
  category: SettingsCategory;
  scope: Scope;
  source: string;
  mtime: string | number | null;
}

const scan = useScanStore();
const ui = useUiStore();
const { search, selectedProjectPath } = storeToRefs(ui);
const { docs, loading } = useSettingsDocs();

const category = ref<SettingsCategory | "all">("all");

const rows = computed<SettingRow[]>(() => {
  const out: SettingRow[] = [];
  for (const d of docs.value) {
    if (!d.data) continue;
    for (const [key, value] of Object.entries(d.data)) {
      out.push({
        id: `${d.artifact.id}:${key}`,
        key,
        value,
        category: categorizeSetting(key),
        scope: d.artifact.scope,
        source: d.artifact.path,
        mtime: d.artifact.modifiedAt ?? null,
      });
    }
  }
  return out;
});

const categoryCounts = computed(() => {
  const m = new Map<SettingsCategory, number>();
  for (const r of rows.value) m.set(r.category, (m.get(r.category) ?? 0) + 1);
  return m;
});

const visible = computed(() =>
  rows.value
    .filter((r) => category.value === "all" || r.category === category.value)
    .filter((r) => matchesQuery(`${r.key} ${r.source} ${JSON.stringify(r.value) ?? ""}`, search.value))
    .sort(
      (a, b) =>
        a.key.localeCompare(b.key) || SCOPE_PRECEDENCE[b.scope] - SCOPE_PRECEDENCE[a.scope],
    ),
);

const isScalar = (v: unknown) => v === null || ["string", "number", "boolean"].includes(typeof v);
const parseErrors = computed(() => docs.value.filter((d) => d.error));
const sources = computed(() => docs.value.filter((d) => d.data));
const selectedProject = computed(() =>
  scan.projects.find((p) => p.path === selectedProjectPath.value),
);
</script>

<template>
  <div class="flex h-full min-h-0">
    <aside class="flex w-52 shrink-0 flex-col border-r border-line">
      <div class="shrink-0 border-b border-line px-3 py-2"><ProjectSelect all-label="All scopes" /></div>
      <nav class="flex-1 space-y-px overflow-y-auto p-2">
        <button
          type="button"
          class="flex h-7 w-full items-center justify-between rounded-md px-2 text-[13px]"
          :class="category === 'all' ? 'bg-elevated text-fg' : 'text-muted hover:bg-elevated/60 hover:text-fg'"
          @click="category = 'all'"
        >
          All settings <span class="font-mono text-2xs text-subtle">{{ rows.length }}</span>
        </button>
        <button
          v-for="c in SETTINGS_CATEGORIES"
          :key="c"
          type="button"
          class="flex h-7 w-full items-center justify-between rounded-md px-2 text-[13px]"
          :class="category === c ? 'bg-elevated text-fg' : 'text-muted hover:bg-elevated/60 hover:text-fg'"
          @click="category = c"
        >
          {{ CATEGORY_LABEL[c] }}
          <span class="font-mono text-2xs text-subtle">{{ categoryCounts.get(c) ?? 0 }}</span>
        </button>
      </nav>
      <div class="shrink-0 border-t border-line px-3 py-2 text-2xs text-subtle">
        <div class="mb-1 flex items-center gap-1"><Icon name="lock" :size="11" /> Read-only</div>
        <div v-for="d in sources" :key="d.artifact.id" class="flex items-center justify-between gap-2 py-px">
          <ScopeBadge :scope="d.artifact.scope" />
          <span :title="d.artifact.path">{{ relativeTime(d.artifact.modifiedAt) }}</span>
        </div>
      </div>
    </aside>

    <div class="min-w-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-5xl space-y-6 px-6 py-5">
        <div
          v-for="d in parseErrors"
          :key="d.artifact.id"
          class="rounded-md border border-warn/30 bg-warn/10 px-3 py-1.5 text-xs text-warn"
        >
          Could not parse {{ shortenPath(d.artifact.path) }}: {{ d.error }}
        </div>

        <LoadingBlock v-if="(loading || !scan.initialized) && !rows.length" :lines="8" />
        <div v-else-if="!visible.length" class="card h-56">
          <EmptyState
            icon="sliders"
            :title="rows.length ? 'No matching settings' : 'No settings files found'"
            :description="
              rows.length
                ? 'Adjust the category or search.'
                : 'No settings.json / settings.local.json / managed settings were discovered.'
            "
          />
        </div>

        <ul v-else class="card divide-y divide-line">
          <li v-for="r in visible" :key="r.id" class="px-3 py-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-mono text-xs font-medium">{{ r.key }}</span>
              <ScopeBadge :scope="r.scope" />
              <Badge v-if="category === 'all'">{{ CATEGORY_LABEL[r.category] }}</Badge>
              <span class="ml-auto min-w-0 truncate font-mono text-2xs text-subtle" :title="r.source">
                {{ shortenPath(r.source) }}
              </span>
            </div>
            <div class="mt-1.5">
              <span v-if="isScalar(r.value)" class="font-mono text-[11.5px] text-muted">
                {{ r.value === null ? "null" : String(maskIfSecret(r.key, r.value)) }}
              </span>
              <JsonViewer v-else :value="r.value" max-height="260px" />
            </div>
          </li>
        </ul>

        <SettingsDiffPanel v-if="selectedProject" :project-path="selectedProject.path" :query="search" />
        <p v-else class="flex items-center gap-1.5 text-2xs text-subtle">
          <Icon name="eye" :size="11" /> Select a project to also see a settings diff across scopes.
        </p>
      </div>
    </div>
  </div>
</template>
