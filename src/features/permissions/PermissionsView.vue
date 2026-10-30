<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { useSettingsDocs } from "@/composables/useSettingsDocs";
import { matchesQuery } from "@/composables/useSearch";
import { useUiStore } from "@/stores/ui";
import { useScanStore } from "@/stores/scan";
import { SCOPE_LABEL, SCOPE_ORDER, SCOPE_PRECEDENCE } from "@/lib/labels";
import { formatValue, shortenPath } from "@/lib/format";
import type { Scope } from "@/types/config";
import Badge, { type Tone } from "@/components/common/Badge.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import LoadingBlock from "@/components/common/LoadingBlock.vue";
import ProjectSelect from "@/components/common/ProjectSelect.vue";
import ScopeBadge from "@/components/common/ScopeBadge.vue";

type RuleType = "allow" | "ask" | "deny" | "setting";

interface PermRow {
  key: string;
  type: RuleType;
  rule: string;
  scope: Scope;
  source: string;
}

const scan = useScanStore();
const { search } = storeToRefs(useUiStore());
const { docs, loading } = useSettingsDocs();

const typeFilter = ref<RuleType | "all">("all");
const scopeFilter = ref<Scope | "all">("all");

const rows = computed<PermRow[]>(() => {
  const out: PermRow[] = [];
  for (const d of docs.value) {
    const perms = d.data?.permissions;
    if (!perms || typeof perms !== "object") continue;
    const p = perms as Record<string, unknown>;
    const source = d.artifact.path;
    const scope = d.artifact.scope;
    for (const t of ["deny", "ask", "allow"] as const) {
      const list = p[t];
      if (!Array.isArray(list)) continue;
      list.forEach((rule, i) =>
        out.push({ key: `${source}:${t}:${i}`, type: t, rule: formatValue(rule), scope, source }),
      );
    }
    for (const [k, v] of Object.entries(p)) {
      if (k === "allow" || k === "ask" || k === "deny") continue;
      const label = Array.isArray(v) ? v.map(formatValue).join(", ") : formatValue(v);
      out.push({ key: `${source}:${k}`, type: "setting", rule: `${k}: ${label}`, scope, source });
    }
  }
  return out;
});

const TYPE_ORDER: Record<RuleType, number> = { deny: 0, ask: 1, allow: 2, setting: 3 };
const TYPE_TONE: Record<RuleType, Tone> = { deny: "danger", ask: "warn", allow: "ok", setting: "neutral" };

const filtered = computed(() =>
  rows.value
    .filter((r) => (typeFilter.value === "all" || r.type === typeFilter.value))
    .filter((r) => scopeFilter.value === "all" || r.scope === scopeFilter.value)
    .filter((r) => matchesQuery(`${r.rule} ${r.source} ${r.type}`, search.value))
    .sort(
      (a, b) =>
        TYPE_ORDER[a.type] - TYPE_ORDER[b.type] ||
        SCOPE_PRECEDENCE[b.scope] - SCOPE_PRECEDENCE[a.scope] ||
        a.rule.localeCompare(b.rule),
    ),
);

const typeCounts = computed(() => {
  const m: Record<RuleType, number> = { allow: 0, ask: 0, deny: 0, setting: 0 };
  for (const r of rows.value) m[r.type]++;
  return m;
});
const scopes = computed(() => SCOPE_ORDER.filter((s) => rows.value.some((r) => r.scope === s)));
const parseErrors = computed(() => docs.value.filter((d) => d.error));
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-1.5 border-b border-line px-4 py-2">
      <ProjectSelect />
      <div class="flex items-center gap-0.5">
        <button type="button" class="chip" :class="typeFilter === 'all' && 'chip-active'" @click="typeFilter = 'all'">
          Все <span class="font-mono text-2xs text-subtle">{{ rows.length }}</span>
        </button>
        <button
          v-for="t in (['deny', 'ask', 'allow', 'setting'] as const)"
          :key="t"
          type="button"
          class="chip capitalize"
          :class="typeFilter === t && 'chip-active'"
          @click="typeFilter = t"
        >
          {{ t === "setting" ? "Режим и каталоги" : t }}
          <span class="font-mono text-2xs text-subtle">{{ typeCounts[t] }}</span>
        </button>
      </div>
      <div class="flex items-center gap-0.5">
        <button type="button" class="chip" :class="scopeFilter === 'all' && 'chip-active'" @click="scopeFilter = 'all'">
          Все области
        </button>
        <button
          v-for="s in scopes"
          :key="s"
          type="button"
          class="chip"
          :class="scopeFilter === s && 'chip-active'"
          @click="scopeFilter = s"
        >
          {{ SCOPE_LABEL[s] }}
        </button>
      </div>
    </div>

    <div
      v-for="d in parseErrors"
      :key="d.artifact.id"
      class="shrink-0 border-b border-warn/30 bg-warn/10 px-4 py-1 text-xs text-warn"
    >
      Не удалось разобрать {{ shortenPath(d.artifact.path) }}: {{ d.error }}
    </div>

    <div class="min-h-0 flex-1 overflow-auto">
      <LoadingBlock v-if="(loading || !scan.initialized) && !rows.length" :lines="6" />
      <EmptyState
        v-else-if="!filtered.length"
        icon="shield"
        :title="rows.length ? 'Нет подходящих прав' : 'Правила прав доступа не найдены'"
        :description="
          rows.length
            ? 'Измените фильтры или поиск.'
            : 'Ни один файл настроек не задаёт ключ `permissions` в выбранных областях.'
        "
      />
      <table v-else class="w-full border-collapse text-xs">
        <thead>
          <tr class="border-b border-line">
            <th class="th w-20">Тип</th>
            <th class="th">Правило</th>
            <th class="th w-24">Область</th>
            <th class="th">Исходный файл</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-line">
          <tr v-for="r in filtered" :key="r.key" class="hover:bg-elevated/40">
            <td class="td"><Badge :tone="TYPE_TONE[r.type]">{{ r.type === "setting" ? "config" : r.type }}</Badge></td>
            <td class="td break-all font-mono text-[11.5px]">{{ r.rule }}</td>
            <td class="td"><ScopeBadge :scope="r.scope" /></td>
            <td class="td break-all font-mono text-2xs text-subtle" :title="r.source">{{ shortenPath(r.source) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
