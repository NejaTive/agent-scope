<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import { matchesQuery } from "@/composables/useSearch";
import { shortenPath } from "@/lib/format";
import { RU, pluralCount, type RuForms } from "@/lib/plural";
import type { ProjectInfo } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import ProductBadge from "@/components/common/ProductBadge.vue";
import ConfirmDialog from "@/components/common/ConfirmDialog.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Icon from "@/components/common/Icon.vue";
import LoadingBlock from "@/components/common/LoadingBlock.vue";

function hasClaude(p: ProjectInfo) {
  return p.hasClaudeDir || p.hasClaudeMd || p.hasMcpJson;
}
function hasCursor(p: ProjectInfo) {
  return p.hasCursorDir || p.hasCursorMcpJson;
}

const scan = useScanStore();
const router = useRouter();
const { search } = storeToRefs(useUiStore());

const pendingRemove = ref<ProjectInfo | null>(null);
const pendingRemoveRoot = ref<string | null>(null);

const projects = computed(() =>
  [...scan.projects]
    .filter((p) => matchesQuery(`${p.name} ${p.path}`, search.value))
    .sort((a, b) => a.name.localeCompare(b.name)),
);

/** Per-project counts derived from the scan snapshot (ProjectInfo only carries `artifactCount`). */
function counts(p: ProjectInfo) {
  const own = (e: { projectPath?: string | null }) => e.projectPath === p.path;
  const rows: Array<{ key: string; n: number; forms: RuForms }> = [
    { key: "skill", n: scan.skillEntries.filter(own).length, forms: RU.skill },
    { key: "rule", n: scan.rules.filter(own).length, forms: RU.rule },
    { key: "agent", n: scan.agents.filter(own).length, forms: RU.agent },
    { key: "hook", n: scan.hooks.filter(own).length, forms: RU.hook },
    { key: "mcp", n: scan.mcpServers.filter(own).length, forms: RU.mcpServer },
  ];
  return rows.filter((x) => x.n > 0).map((x) => ({ key: x.key, text: pluralCount(x.n, x.forms) }));
}

function open(p: ProjectInfo) {
  void router.push({ name: "project-detail", params: { id: encodeURIComponent(p.path) } });
}

async function confirmRemoveProject() {
  const p = pendingRemove.value;
  pendingRemove.value = null;
  if (p) await scan.removeProject(p.path);
}

async function confirmRemoveRoot() {
  const path = pendingRemoveRoot.value;
  pendingRemoveRoot.value = null;
  if (path) await scan.removeScanRoot(path);
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
            <button
              type="button"
              class="btn-ghost !h-6"
              title="Убрать корень сканирования из приложения"
              @click="pendingRemoveRoot = r.path"
            >
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
                <ProductBadge v-if="hasClaude(p)" product="claude" />
                <ProductBadge v-if="hasCursor(p)" product="cursor" />
              </span>
              <span class="truncate font-mono text-2xs text-subtle" :title="p.path">{{ shortenPath(p.path) }}</span>
            </button>
            <div class="flex items-center gap-1">
              <Badge v-for="c in counts(p)" :key="c.key" mono>{{ c.text }}</Badge>
            </div>
            <button
              type="button"
              class="btn-ghost !h-6 opacity-0 group-hover:opacity-100"
              title="Убрать из списка приложения (файлы не удаляются)"
              @click.stop="pendingRemove = p"
            >
              <Icon name="x" :size="12" />
            </button>
          </li>
        </ul>
      </section>
    </div>

    <ConfirmDialog
      :open="!!pendingRemove"
      title="Убрать проект из списка?"
      :description="
        pendingRemove
          ? `${pendingRemove.name}\n${shortenPath(pendingRemove.path)}\n\nПроект только скроется из Agent Scope. Файлы на диске не удаляются.`
          : ''
      "
      confirm-label="Убрать"
      cancel-label="Отмена"
      danger
      @confirm="confirmRemoveProject"
      @cancel="pendingRemove = null"
    />

    <ConfirmDialog
      :open="!!pendingRemoveRoot"
      title="Убрать корень сканирования?"
      :description="
        pendingRemoveRoot
          ? `${shortenPath(pendingRemoveRoot)}\n\nКорень только уберётся из Agent Scope. Файлы и проекты на диске не удаляются.`
          : ''
      "
      confirm-label="Убрать"
      cancel-label="Отмена"
      danger
      @confirm="confirmRemoveRoot"
      @cancel="pendingRemoveRoot = null"
    />
  </div>
</template>
