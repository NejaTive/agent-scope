<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import { matchesQuery } from "@/composables/useSearch";
import { shortenPath } from "@/lib/format";
import { PRODUCT_DIR_LABEL } from "@/lib/labels";
import { pluralCountLocale, type PluralKey } from "@/lib/plural";
import { projectHasProduct, trackedProviders } from "@/lib/tooling";
import type { ProjectInfo } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import ProductBadge from "@/components/common/ProductBadge.vue";
import ConfirmDialog from "@/components/common/ConfirmDialog.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Icon from "@/components/common/Icon.vue";
import LoadingBlock from "@/components/common/LoadingBlock.vue";

const { t } = useI18n();
const scan = useScanStore();
const prefs = usePreferencesStore();
const router = useRouter();
const { search } = storeToRefs(useUiStore());

function joinList(items: string[], conj: string): string {
  if (items.length === 0) return "";
  if (items.length === 1) return items[0]!;
  return `${items.slice(0, -1).join(", ")} ${conj} ${items[items.length - 1]}`;
}

const trackedDirs = computed(() =>
  trackedProviders(prefs.prefs).map((p) => PRODUCT_DIR_LABEL[p.id]),
);

const emptyProjectsDescription = computed(() => {
  if (scan.projects.length) return t("projects.emptyFiltered");
  const labels = trackedProviders(prefs.prefs).map((p) => p.label);
  const products = labels.length
    ? joinList(labels, t("common.and"))
    : t("projects.emptyProductsFallback");
  return t("projects.emptyNone", { products });
});

const scanRootsEmptyDirs = computed(() =>
  joinList(trackedDirs.value, t("common.or")),
);

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
  const rows: Array<{ key: string; n: number; plural: PluralKey }> = [
    { key: "skill", n: scan.skillEntries.filter(own).length, plural: "skill" },
    { key: "rule", n: scan.rules.filter(own).length, plural: "rule" },
    { key: "agent", n: scan.agents.filter(own).length, plural: "agent" },
    { key: "hook", n: scan.hooks.filter(own).length, plural: "hook" },
    { key: "mcp", n: scan.mcpServers.filter(own).length, plural: "mcpServer" },
  ];
  return rows
    .filter((x) => x.n > 0)
    .map((x) => ({ key: x.key, text: pluralCountLocale(x.n, x.plural) }));
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
          <Icon name="plus" :size="13" /> {{ t("projects.addProject") }}
        </button>
        <button type="button" class="btn" @click="scan.addScanRootFromPicker()">
          <Icon name="folder" :size="13" /> {{ t("projects.addScanRoot") }}
        </button>
        <span class="ml-auto flex items-center gap-1 text-2xs text-subtle">
          <Icon name="lock" :size="11" />
          {{ t("projects.removeHint") }}
        </span>
      </section>

      <section>
        <h2 class="label mb-2">{{ t("projects.scanRoots", { count: scan.scanRoots.length }) }}</h2>
        <div v-if="!scan.scanRoots.length" class="card px-3 py-2 text-xs text-muted">
          {{
            t("projects.scanRootsEmpty", {
              example: "~/Projects",
              dirs: scanRootsEmptyDirs,
            })
          }}
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
              :title="t('projects.removeRootTitle')"
              @click="pendingRemoveRoot = r.path"
            >
              <Icon name="x" :size="12" />
            </button>
          </li>
        </ul>
      </section>

      <section>
        <h2 class="label mb-2">{{ t("projects.list", { count: projects.length }) }}</h2>
        <LoadingBlock v-if="!scan.initialized" :lines="4" />
        <div v-else-if="!projects.length" class="card h-48">
          <EmptyState
            icon="folder"
            :title="scan.projects.length ? t('projects.noMatching') : t('projects.noneYet')"
            :description="emptyProjectsDescription"
          />
        </div>
        <ul v-else class="card divide-y divide-line">
          <li v-for="p in projects" :key="p.path" class="group flex items-center gap-3 px-3 py-2 hover:bg-elevated/40">
            <button type="button" class="flex min-w-0 flex-1 flex-col gap-0.5 text-left" @click="open(p)">
              <span class="flex items-center gap-2">
                <span class="truncate text-[13px] font-medium">{{ p.name }}</span>
                <ProductBadge
                  v-for="prov in trackedProviders(prefs.prefs).filter((x) => projectHasProduct(p, x.id))"
                  :key="prov.id"
                  :product="prov.id"
                />
              </span>
              <span class="truncate font-mono text-2xs text-subtle" :title="p.path">{{ shortenPath(p.path) }}</span>
            </button>
            <div class="flex items-center gap-1">
              <Badge v-for="c in counts(p)" :key="c.key" mono>{{ c.text }}</Badge>
            </div>
            <button
              type="button"
              class="btn-ghost !h-6 opacity-0 group-hover:opacity-100"
              :title="t('projects.removeProjectTitle')"
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
      :title="t('projects.confirmRemoveProject')"
      :description="
        pendingRemove
          ? t('projects.confirmRemoveProjectDesc', {
              name: pendingRemove.name,
              path: shortenPath(pendingRemove.path),
            })
          : ''
      "
      :confirm-label="t('common.remove')"
      :cancel-label="t('common.cancel')"
      danger
      @confirm="confirmRemoveProject"
      @cancel="pendingRemove = null"
    />

    <ConfirmDialog
      :open="!!pendingRemoveRoot"
      :title="t('projects.confirmRemoveRoot')"
      :description="
        pendingRemoveRoot
          ? t('projects.confirmRemoveRootDesc', { path: shortenPath(pendingRemoveRoot) })
          : ''
      "
      :confirm-label="t('common.remove')"
      :cancel-label="t('common.cancel')"
      danger
      @confirm="confirmRemoveRoot"
      @cancel="pendingRemoveRoot = null"
    />
  </div>
</template>
