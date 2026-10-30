<script setup lang="ts">
import { computed, ref } from "vue";
import { RouterLink, useRoute } from "vue-router";
import { storeToRefs } from "pinia";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import { artifactToItem, type BrowserItem } from "@/lib/browser";
import { shortenPath } from "@/lib/format";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import Badge from "@/components/common/Badge.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Icon from "@/components/common/Icon.vue";
import SettingsDiffPanel from "@/features/settings/SettingsDiffPanel.vue";

const route = useRoute();
const scan = useScanStore();
const { search } = storeToRefs(useUiStore());

const tab = ref<"inventory" | "diff">("inventory");

const projectId = computed(() => decodeURIComponent(String(route.params.id ?? "")));
const project = computed(() =>
  scan.projects.find((p) => p.path === projectId.value),
);

const items = computed<BrowserItem[]>(() => {
  const path = project.value?.path;
  if (!path) return [];
  const out = new Map<string, BrowserItem>();
  const artifactPaths = new Set<string>();
  for (const a of scan.artifacts) {
    if (a.projectPath !== path) continue;
    out.set(a.id, artifactToItem(a));
    artifactPaths.add(`${a.kind}:${a.path}`);
  }
  // Hooks / MCP servers are listed individually unless the backend already emitted
  // an artifact of that kind for the same declaring file.
  for (const h of scan.hooks) {
    if (h.projectPath !== path || out.has(h.id) || artifactPaths.has(`hook:${h.sourcePath}`)) continue;
    out.set(h.id, {
      id: h.id,
      name: h.matcher ? `${h.event} · ${h.matcher}` : h.event,
      subtitle: h.command ?? h.hookType,
      scope: h.scope,
      status: h.status,
      mtime: h.mtime,
      path: h.sourcePath,
      projectPath: h.projectPath,
      artifactId: h.sourcePath,
      kind: "hook",
    });
  }
  for (const s of scan.mcpServers) {
    if (s.projectPath !== path || out.has(s.id) || artifactPaths.has(`mcpServer:${s.sourcePath}`)) continue;
    out.set(s.id, {
      id: s.id,
      name: s.name,
      subtitle: s.url ?? s.command ?? null,
      scope: s.scope,
      status: s.status,
      mtime: s.mtime,
      path: s.sourcePath,
      projectPath: s.projectPath,
      artifactId: s.sourcePath,
      kind: "mcpServer",
    });
  }
  return [...out.values()];
});
</script>

<template>
  <div v-if="!project" class="h-full">
    <EmptyState
      icon="folder"
      title="Проект не найден"
      description="Этот проект больше не в списке проектов приложения."
    >
      <RouterLink to="/projects" class="btn">К проектам</RouterLink>
    </EmptyState>
  </div>

  <div v-else class="flex h-full min-h-0 flex-col">
    <div class="flex shrink-0 items-center gap-3 border-b border-line px-4 py-2">
      <RouterLink to="/projects" class="btn-ghost !px-1.5" title="К проектам">
        <Icon name="chevron-right" :size="14" class="rotate-180" />
      </RouterLink>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-2">
          <span class="truncate text-[13px] font-semibold">{{ project.name }}</span>
          <Badge v-if="project.pinned" tone="accent">закреплён</Badge>
          <Badge v-if="!project.hasClaudeDir">нет .claude</Badge>
        </div>
        <div class="truncate font-mono text-2xs text-subtle" :title="project.path">
          {{ shortenPath(project.path) }}
        </div>
      </div>
      <div class="flex items-center gap-0.5" role="tablist">
        <button type="button" class="chip" :class="tab === 'inventory' && 'chip-active'" @click="tab = 'inventory'">
          Инвентарь <span class="font-mono text-2xs text-subtle">{{ items.length }}</span>
        </button>
        <button type="button" class="chip" :class="tab === 'diff' && 'chip-active'" @click="tab = 'diff'">
          Diff настроек
        </button>
      </div>
    </div>

    <div class="min-h-0 flex-1">
      <ArtifactBrowser
        v-if="tab === 'inventory'"
        :items="items"
        :loading="!scan.initialized"
        kind-filter
        empty-title="Нет проектной конфигурации"
        empty-description="В этом проекте не найдены каталог .claude, CLAUDE.md или .mcp.json."
      />
      <div v-else class="h-full overflow-y-auto">
        <div class="mx-auto max-w-5xl px-6 py-5">
          <SettingsDiffPanel :project-path="project.path" :query="search" />
        </div>
      </div>
    </div>
  </div>
</template>
