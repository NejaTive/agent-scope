<script setup lang="ts">
import { computed, watch } from "vue";
import { useInvoke } from "@/composables/useInvoke";
import { matchesQuery } from "@/composables/useSearch";
import { ipc } from "@/lib/ipc";
import { formatValue, shortenPath } from "@/lib/format";
import { maskIfSecret } from "@/lib/secrets";
import type { SettingsDiff, SettingsDiffRow } from "@/types/config";
import { useScanStore } from "@/stores/scan";
import Badge from "@/components/common/Badge.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import LoadingBlock from "@/components/common/LoadingBlock.vue";

const props = defineProps<{ projectPath: string; query?: string }>();

type Column = "user" | "project" | "local";
const COLUMNS: Array<{ key: Column; label: string; pathKey: keyof SettingsDiff }> = [
  { key: "user", label: "User", pathKey: "userPath" },
  { key: "project", label: "Project", pathKey: "projectSettingsPath" },
  { key: "local", label: "Local", pathKey: "localPath" },
];
/** Highest precedence first (managed is not part of the diff). */
const PRECEDENCE: Column[] = ["local", "project", "user"];

const scan = useScanStore();
const diff = useInvoke((p: string) => ipc.getSettingsDiff(p), null as SettingsDiff | null);

watch(
  [() => props.projectPath, () => scan.snapshot],
  () => void diff.run(props.projectPath),
  { immediate: true },
);

const rows = computed<SettingsDiffRow[]>(() =>
  (diff.data.value?.rows ?? [])
    .filter((r) => matchesQuery(r.key, props.query ?? ""))
    .sort((a, b) => Number(b.differs) - Number(a.differs) || a.key.localeCompare(b.key)),
);
const totalRows = computed(() => diff.data.value?.rows.length ?? 0);

const has = (v: unknown) => v !== undefined && v !== null;

function cell(r: SettingsDiffRow, c: Column): string | null {
  const v = r[c];
  return has(v) ? formatValue(maskIfSecret(r.key, v)) : null;
}

/** Column whose value wins (highest precedence that defines the key). */
function winner(r: SettingsDiffRow): Column | null {
  return PRECEDENCE.find((c) => has(r[c])) ?? null;
}

function columnPath(c: (typeof COLUMNS)[number]): string | null {
  const p = diff.data.value?.[c.pathKey];
  return typeof p === "string" ? p : null;
}
</script>

<template>
  <section>
    <div class="mb-2 flex items-center gap-2">
      <h2 class="label">Settings diff across scopes</h2>
      <span class="font-mono text-2xs text-subtle">{{ rows.length }} keys</span>
    </div>
    <LoadingBlock v-if="diff.loading.value && !diff.data.value" :lines="4" />
    <div v-else-if="diff.error.value" class="card h-40">
      <EmptyState compact tone="danger" title="Could not compute settings diff" :description="diff.error.value" />
    </div>
    <div v-else-if="!rows.length" class="card h-32">
      <EmptyState
        compact
        :title="totalRows ? 'No matching keys' : 'No settings to compare'"
        :description="totalRows ? 'Adjust the search.' : 'No user, project or local settings define any keys for this project.'"
      />
    </div>
    <div v-else class="card overflow-x-auto">
      <table class="w-full border-collapse text-xs">
        <thead>
          <tr class="border-b border-line">
            <th class="th">Key</th>
            <th v-for="c in COLUMNS" :key="c.key" class="th">
              {{ c.label }}
              <span
                v-if="columnPath(c)"
                class="block max-w-[220px] truncate font-mono text-2xs font-normal normal-case text-subtle"
                :title="columnPath(c) ?? undefined"
              >
                {{ shortenPath(columnPath(c)!) }}
              </span>
            </th>
            <th class="th">Status</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-line">
          <tr v-for="r in rows" :key="r.key">
            <td class="td whitespace-nowrap font-mono text-[11.5px]">{{ r.key }}</td>
            <td v-for="c in COLUMNS" :key="c.key" class="td max-w-[260px]">
              <span
                v-if="cell(r, c.key) !== null"
                class="block break-all font-mono text-[11.5px]"
                :class="r.differs && winner(r) === c.key ? 'text-accent' : 'text-muted'"
                :title="r.differs && winner(r) === c.key ? 'Winning value' : undefined"
              >
                {{ cell(r, c.key) }}
              </span>
              <span v-else class="text-subtle">—</span>
            </td>
            <td class="td">
              <Badge :tone="r.differs ? 'warn' : 'neutral'">{{ r.differs ? "differs" : "same" }}</Badge>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
