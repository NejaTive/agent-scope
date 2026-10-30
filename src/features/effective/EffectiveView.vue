<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useInvoke } from "@/composables/useInvoke";
import { matchesQuery } from "@/composables/useSearch";
import { ipc } from "@/lib/ipc";
import { formatValue, shortenPath } from "@/lib/format";
import { maskIfSecret } from "@/lib/secrets";
import { SCOPE_LABEL } from "@/lib/labels";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import type { Confidence, ResolvedSetting } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import ConfidenceBadge from "@/components/common/ConfidenceBadge.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import LoadingBlock from "@/components/common/LoadingBlock.vue";
import ProjectSelect from "@/components/common/ProjectSelect.vue";
import ScopeBadge from "@/components/common/ScopeBadge.vue";

const scan = useScanStore();
const { search, selectedProjectPath } = storeToRefs(useUiStore());

const settings = useInvoke((p: string | null) => ipc.getEffectiveSettings(p), [] as ResolvedSetting[]);
// `get_conflicts` takes no arguments: filter client-side to the selected project
// (conflicts without a projectPath are global and always shown).
const conflicts = computed(() =>
  scan.conflicts.filter(
    (c) => !c.projectPath || !selectedProjectPath.value || c.projectPath === selectedProjectPath.value,
  ),
);

watch(
  [selectedProjectPath, () => scan.snapshot],
  () => {
    void settings.run(selectedProjectPath.value);
  },
  { immediate: true },
);

const confidence = ref<Confidence | "all">("all");
const expanded = ref<Set<string>>(new Set());

function toggle(key: string) {
  const next = new Set(expanded.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  expanded.value = next;
}

const rows = computed(() =>
  settings.data.value
    .filter((r) => confidence.value === "all" || r.confidence === confidence.value)
    .filter((r) => matchesQuery(`${r.key} ${formatValue(r.value)} ${r.explanation} ${r.sourcePath ?? ""}`, search.value))
    .sort((a, b) => a.key.localeCompare(b.key)),
);

const CONFIDENCES: Confidence[] = ["computed", "inferred", "unknown"];
const CONFIDENCE_LABEL: Record<Confidence, string> = {
  computed: "Вычислено",
  inferred: "Выведено",
  unknown: "Неизвестно",
};
const confCounts = computed(() => {
  const m = new Map<Confidence, number>();
  for (const r of settings.data.value) m.set(r.confidence, (m.get(r.confidence) ?? 0) + 1);
  return m;
});

const display = (r: ResolvedSetting) => formatValue(maskIfSecret(r.key, r.value));
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-1.5 border-b border-line px-4 py-2">
      <ProjectSelect all-label="Только пользователь + управляемый" />
      <div class="flex items-center gap-0.5">
        <button type="button" class="chip" :class="confidence === 'all' && 'chip-active'" @click="confidence = 'all'">
          Все <span class="font-mono text-2xs text-subtle">{{ settings.data.value.length }}</span>
        </button>
        <button
          v-for="c in CONFIDENCES.filter((x) => confCounts.has(x))"
          :key="c"
          type="button"
          class="chip"
          :class="confidence === c && 'chip-active'"
          @click="confidence = c"
        >
          {{ CONFIDENCE_LABEL[c] }} <span class="font-mono text-2xs text-subtle">{{ confCounts.get(c) }}</span>
        </button>
      </div>
      <span class="ml-auto text-2xs text-subtle">
        Разрешение по приоритету: управляемый › локальный › проект › пользователь. Уверенность: вычислено = точное слияние, выведено = лучшая оценка.
      </span>
    </div>

    <div class="min-h-0 flex-1 overflow-auto">
      <section v-if="conflicts.length" class="border-b border-line bg-warn/5 px-4 py-2">
        <div class="label mb-1 !text-warn">Конфликты ({{ conflicts.length }})</div>
        <ul class="space-y-1">
          <li v-for="c in conflicts" :key="c.id" class="flex items-start gap-2 text-xs">
            <Badge :tone="c.severity === 'info' ? 'info' : 'warn'">{{ c.severity }}</Badge>
            <span class="font-mono">{{ c.key }}</span>
            <span class="min-w-0 flex-1 text-muted">{{ c.message }}</span>
            <span class="flex gap-1">
              <ScopeBadge v-for="(e, j) in c.entries" :key="`${e.path}-${j}`" :scope="e.scope" :title="e.path" />
            </span>
          </li>
        </ul>
      </section>

      <LoadingBlock v-if="settings.loading.value && !settings.data.value.length" :lines="8" />
      <EmptyState
        v-else-if="settings.error.value"
        tone="danger"
        title="Не удалось разрешить эффективную конфигурацию"
        :description="settings.error.value"
      />
      <EmptyState
        v-else-if="!rows.length"
        icon="layers"
        :title="settings.data.value.length ? 'Нет подходящих настроек' : 'Нет эффективной конфигурации'"
        :description="
          settings.data.value.length
            ? 'Измените фильтр уверенности или поиск.'
            : 'Настройки не разрешены. Запустите сканирование или выберите проект.'
        "
      />
      <table v-else class="w-full border-collapse text-xs">
        <thead>
          <tr class="border-b border-line">
            <th class="th w-6" />
            <th class="th">Ключ</th>
            <th class="th">Значение</th>
            <th class="th w-24">Источник</th>
            <th class="th w-28">Уверенность</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-line">
          <template v-for="r in rows" :key="r.key">
            <tr class="cursor-pointer hover:bg-elevated/40" @click="toggle(r.key)">
              <td class="td pr-0 text-subtle">
                <span class="inline-block transition-transform" :class="expanded.has(r.key) && 'rotate-90'">›</span>
              </td>
              <td class="td whitespace-nowrap font-mono text-[11.5px]">{{ r.key }}</td>
              <td class="td max-w-[360px] break-all font-mono text-[11.5px] text-muted">{{ display(r) }}</td>
              <td class="td"><ScopeBadge :scope="r.scope" /></td>
              <td class="td"><ConfidenceBadge :confidence="r.confidence" /></td>
            </tr>
            <tr v-if="expanded.has(r.key)" class="bg-elevated/30">
              <td />
              <td colspan="4" class="space-y-1.5 px-3 py-2">
                <p class="text-xs text-muted">{{ r.explanation || "Пояснение не указано." }}</p>
                <p v-if="r.sourcePath" class="break-all font-mono text-2xs text-subtle">
                  {{ shortenPath(r.sourcePath) }}
                </p>
                <div v-if="r.sources.length">
                  <div class="label mb-1 flex items-center gap-1.5">
                    Источники (сначала высший приоритет)
                    <Badge v-if="r.merged" tone="info">слияние</Badge>
                  </div>
                  <ul class="space-y-0.5">
                    <li v-for="(o, i) in r.sources" :key="`${o.path}-${i}`" class="flex items-center gap-2 text-xs">
                      <span class="w-16 text-subtle">{{ SCOPE_LABEL[o.scope] }}</span>
                      <span
                        class="font-mono text-[11.5px]"
                        :class="i === 0 || r.merged ? 'text-fg' : 'text-muted line-through decoration-subtle/50'"
                      >
                        {{ formatValue(maskIfSecret(r.key, o.value)) }}
                      </span>
                      <span class="min-w-0 truncate font-mono text-2xs text-subtle" :title="o.path">
                        {{ shortenPath(o.path) }}
                      </span>
                    </li>
                  </ul>
                </div>
              </td>
            </tr>
          </template>
        </tbody>
      </table>
    </div>
  </div>
</template>
