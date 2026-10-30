<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import { formatTime, relativeTime, shortenPath } from "@/lib/format";
import Badge from "@/components/common/Badge.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import ScopeBadge from "@/components/common/ScopeBadge.vue";

const scan = useScanStore();
const ui = useUiStore();
const router = useRouter();

onMounted(() => ui.markChangesSeen());

const cards = computed(() => {
  const d = scan.dashboard;
  return [
    { label: "Projects", value: d.projectCount, to: "/projects" },
    { label: "Global skills", value: d.globalSkills, to: "/skills" },
    { label: "Project skills", value: d.projectSkills, to: "/skills" },
    { label: "Rules", value: d.rules, to: "/rules" },
    { label: "Agents", value: d.agents, to: "/agents" },
    { label: "Hooks", value: d.hooks, to: "/hooks" },
    { label: "MCP servers", value: d.mcpServers, to: "/mcp" },
    { label: "Plugins", value: d.plugins, to: "/plugins" },
    { label: "Commands", value: d.commands, to: "/skills" },
    { label: "Memory files", value: d.memories, to: "/memory" },
    { label: "Conflicts", value: d.conflictCount, to: "/effective", warn: d.conflictCount > 0 },
  ];
});

const CONFLICT_KIND_LABEL: Record<string, string> = {
  duplicateName: "duplicate name",
  mcpMismatch: "MCP mismatch",
  settingOverride: "setting override",
};

const CHANGE_TONE: Record<string, "ok" | "info" | "danger"> = {
  created: "ok",
  create: "ok",
  modified: "info",
  modify: "info",
  changed: "info",
  deleted: "danger",
  removed: "danger",
  remove: "danger",
};
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="mx-auto max-w-5xl space-y-6 px-6 py-5">
      <section>
        <div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-5">
          <button
            v-for="c in cards"
            :key="c.label"
            type="button"
            class="card group px-3 py-2.5 text-left transition-colors hover:border-subtle/50 hover:bg-elevated/40"
            @click="router.push(c.to)"
          >
            <div class="label">{{ c.label }}</div>
            <div
              class="mt-1 font-mono text-2xl font-semibold tabular-nums"
              :class="c.warn ? 'text-warn' : 'text-fg'"
            >
              {{ c.value }}
            </div>
          </button>
          <div class="card px-3 py-2.5">
            <div class="label">Last scan</div>
            <div class="mt-1 text-[13px] font-medium">{{ relativeTime(scan.lastScanAt) }}</div>
            <div class="text-2xs text-subtle">
              {{ formatTime(scan.lastScanAt) }}
              <template v-if="scan.snapshot.durationMs != null"> · {{ scan.snapshot.durationMs }} ms</template>
            </div>
          </div>
        </div>
      </section>

      <section v-if="scan.scanErrors.length">
        <h2 class="label mb-2">Scan errors ({{ scan.scanErrors.length }})</h2>
        <ul class="card divide-y divide-line">
          <li v-for="(e, i) in scan.scanErrors" :key="i" class="flex flex-col gap-0.5 px-3 py-1.5">
            <span class="break-all font-mono text-[11.5px] text-muted">{{ shortenPath(e.path) }}</span>
            <span class="text-xs text-danger">{{ e.message }}</span>
          </li>
        </ul>
      </section>

      <section v-if="scan.conflicts.length">
        <h2 class="label mb-2">Conflicts ({{ scan.conflicts.length }})</h2>
        <ul class="card divide-y divide-line">
          <li v-for="(c, i) in scan.conflicts" :key="c.id ?? i" class="flex items-start gap-2 px-3 py-1.5">
            <Badge :tone="c.severity === 'info' ? 'info' : 'warn'">{{ c.severity }}</Badge>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <span class="font-mono text-xs">{{ c.key }}</span>
                <Badge mono>{{ CONFLICT_KIND_LABEL[c.kind] ?? c.kind }}</Badge>
              </div>
              <div class="text-xs text-muted">{{ c.message }}</div>
            </div>
            <div class="flex flex-wrap justify-end gap-1">
              <ScopeBadge v-for="(e, j) in c.entries" :key="`${e.path}-${j}`" :scope="e.scope" :title="e.path" />
            </div>
          </li>
        </ul>
      </section>

      <section>
        <h2 class="label mb-2">Recent changes</h2>
        <div v-if="!scan.recentChanges.length" class="card h-40">
          <EmptyState
            compact
            icon="activity"
            title="No recent changes"
            description="File changes under watched Claude config locations will appear here."
          />
        </div>
        <ul v-else class="card divide-y divide-line">
          <li
            v-for="(c, i) in scan.recentChanges"
            :key="c.id ?? `${c.path}-${i}`"
            class="flex items-center gap-2 px-3 py-1.5"
          >
            <Badge :tone="CHANGE_TONE[c.kind] ?? 'neutral'">{{ c.kind }}</Badge>
            <span class="min-w-0 flex-1 truncate font-mono text-[11.5px]" :title="c.path">
              {{ shortenPath(c.path) }}
            </span>
            <span class="w-16 shrink-0 text-right text-2xs text-subtle" :title="formatTime(c.ts)">
              {{ relativeTime(c.ts) }}
            </span>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>
