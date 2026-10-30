<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import { matchesQuery } from "@/composables/useSearch";
import { shortenPath } from "@/lib/format";
import type { ProjectInfo } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Icon from "@/components/common/Icon.vue";
import LoadingBlock from "@/components/common/LoadingBlock.vue";

const scan = useScanStore();
const router = useRouter();
const { search } = storeToRefs(useUiStore());

const confirming = ref<string | null>(null);
const confirmingRoot = ref<string | null>(null);

const projects = computed(() =>
  [...scan.projects]
    .filter((p) => matchesQuery(`${p.name} ${p.path}`, search.value))
    .sort((a, b) => a.name.localeCompare(b.name)),
);

/** Per-project counts derived from the scan snapshot (ProjectInfo only carries `artifactCount`). */
function counts(p: ProjectInfo) {
  const own = (e: { projectPath?: string | null }) => e.projectPath === p.path;
  return [
    { label: "скиллы", n: scan.skillEntries.filter(own).length },
    { label: "правила", n: scan.rules.filter(own).length },
    { label: "агенты", n: scan.agents.filter(own).length },
    { label: "хуки", n: scan.hooks.filter(own).length },
    { label: "mcp", n: scan.mcpServers.filter(own).length },
  ].filter((x) => x.n > 0);
}

function open(p: ProjectInfo) {
  void router.push({ name: "project-detail", params: { id: encodeURIComponent(p.path) } });
}

async function remove(p: ProjectInfo) {
  confirming.value = null;
  await scan.removeProject(p.path);
}

async function removeRoot(path: string) {
  confirmingRoot.value = null;
  await scan.removeScanRoot(path);
}
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="mx-auto max-w-5xl space-y-6 px-6 py-5">
      <section class="flex flex-wrap items-center gap-2">
        <button type="button" class="btn-accent" @click="scan.addProjectFromPicker()">
          <Icon name="plus" :size="13" /> Добавить проект…
        </button>
        <button type="button" class="btn" @click="scan.addScanRootFromPicker()">
          <Icon name="folder" :size="13" /> Добавить корень сканирования…
        </button>
        <span class="ml-auto flex items-center gap-1 text-2xs text-subtle">
          <Icon name="lock" :size="11" />
          Удаление только скрывает проект из приложения — файлы на диске не затрагиваются.
        </span>
      </section>

      <section>
        <h2 class="label mb-2">Корни сканирования ({{ scan.scanRoots.length }})</h2>
        <div v-if="!scan.scanRoots.length" class="card px-3 py-2 text-xs text-muted">
          Нет корней сканирования. Добавьте папку (например <span class="font-mono">~/Projects</span>), чтобы автоматически находить проекты
          с <span class="font-mono">.claude</span> или <span class="font-mono">.cursor</span>.
        </div>
        <ul v-else class="card divide-y divide-line">
          <li v-for="r in scan.scanRoots" :key="r.path" class="flex items-center gap-2 px-3 py-1.5">
            <Icon name="folder" :size="13" class="text-subtle" />
            <span class="min-w-0 flex-1 truncate font-mono text-[11.5px]" :title="r.path">
              {{ shortenPath(r.path) }}
            </span>
            <template v-if="confirmingRoot === r.path">
              <button type="button" class="btn !h-6 text-danger" @click="removeRoot(r.path)">Убрать из списка</button>
              <button type="button" class="btn-ghost !h-6" @click="confirmingRoot = null">Отмена</button>
            </template>
            <button v-else type="button" class="btn-ghost !h-6" title="Убрать корень сканирования из приложения" @click="confirmingRoot = r.path">
              <Icon name="x" :size="12" />
            </button>
          </li>
        </ul>
      </section>

      <section>
        <h2 class="label mb-2">Проекты ({{ projects.length }})</h2>
        <LoadingBlock v-if="!scan.initialized" :lines="4" />
        <div v-else-if="!projects.length" class="card h-48">
          <EmptyState
            icon="folder"
            :title="scan.projects.length ? 'Нет подходящих проектов' : 'Пока нет проектов'"
            :description="
              scan.projects.length
                ? 'Очистите поиск, чтобы увидеть все проекты.'
                : 'Добавьте папку проекта или корень сканирования, чтобы начать просмотр конфигурации Claude Code и Cursor.'
            "
          />
        </div>
        <ul v-else class="card divide-y divide-line">
          <li v-for="p in projects" :key="p.path" class="group flex items-center gap-3 px-3 py-2 hover:bg-elevated/40">
            <button type="button" class="flex min-w-0 flex-1 flex-col gap-0.5 text-left" @click="open(p)">
              <span class="flex items-center gap-2">
                <span class="truncate text-[13px] font-medium">{{ p.name }}</span>
                <Badge v-if="p.pinned" tone="accent">закреплён</Badge>
                <Badge v-if="p.discovered && !p.pinned">обнаружен</Badge>
                <Badge v-if="p.hasClaudeDir" mono tone="info">.claude</Badge>
                <Badge v-if="p.hasCursorDir" mono tone="accent">.cursor</Badge>
                <Badge v-if="!p.hasClaudeDir && !p.hasCursorDir">нет .claude / .cursor</Badge>
                <Badge v-if="p.hasClaudeMd" mono>CLAUDE.md</Badge>
                <Badge v-if="p.hasMcpJson" mono>.mcp.json</Badge>
                <Badge v-if="p.hasCursorMcpJson" mono>.cursor/mcp.json</Badge>
              </span>
              <span class="truncate font-mono text-2xs text-subtle" :title="p.path">{{ shortenPath(p.path) }}</span>
            </button>
            <div class="flex items-center gap-1">
              <Badge v-for="c in counts(p)" :key="c.label" mono>{{ c.n }} {{ c.label }}</Badge>
            </div>
            <span class="w-24 shrink-0 text-right text-2xs text-subtle">{{ p.artifactCount }} артефактов</span>
            <div class="flex items-center gap-1">
              <button type="button" class="btn !h-6" @click="open(p)">Инвентарь</button>
              <template v-if="confirming === p.path">
                <button type="button" class="btn !h-6 text-danger" @click="remove(p)">Убрать из списка</button>
                <button type="button" class="btn-ghost !h-6" @click="confirming = null">Отмена</button>
              </template>
              <button
                v-else
                type="button"
                class="btn-ghost !h-6 opacity-0 group-hover:opacity-100"
                title="Убрать из списка приложения (файлы не удаляются)"
                @click="confirming = p.path"
              >
                <Icon name="x" :size="12" />
              </button>
            </div>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>
