<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ipc } from "@/lib/ipc";
import { errMsg, normalizeContent } from "@/lib/normalize";
import { extname, formatTime, shortenPath } from "@/lib/format";
import { languageForExt } from "@/lib/highlight";
import { KIND_LABEL, STATUS_LABEL, STATUS_TONE } from "@/lib/labels";
import type { BrowserItem } from "@/lib/browser";
import type { ViewContent } from "@/types/config";
import { useScanStore } from "@/stores/scan";
import Badge from "./Badge.vue";
import ScopeBadge from "./ScopeBadge.vue";
import LoadingBlock from "./LoadingBlock.vue";
import EmptyState from "./EmptyState.vue";
import Icon from "./Icon.vue";
import MarkdownViewer from "@/components/viewers/MarkdownViewer.vue";
import JsonViewer from "@/components/viewers/JsonViewer.vue";
import CodeViewer from "@/components/viewers/CodeViewer.vue";

const props = withDefaults(
  defineProps<{
    item: BrowserItem;
    /** Hide the raw file behind a toggle (for structured views like hooks / MCP). */
    collapsedContent?: boolean;
    contentLabel?: string;
  }>(),
  { collapsedContent: false, contentLabel: "File content" },
);

const scan = useScanStore();
const content = ref<ViewContent | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const showContent = ref(!props.collapsedContent);
let token = 0;

/** `get_artifact_content` takes a filesystem path, never an artifact id. */
const contentPath = computed(() => props.item.artifactId ?? props.item.path);
const ext = computed(() => extname(props.item.path));
const mode = computed<"markdown" | "json" | "code">(() => {
  const lang = content.value?.language;
  if (lang === "markdown" || ext.value === "md" || ext.value === "mdc" || ext.value === "markdown")
    return "markdown";
  if (lang === "json" || ext.value === "json" || ext.value === "jsonc") return "json";
  return "code";
});
const language = computed(() => {
  const lang = content.value?.language;
  if (lang && lang !== "plaintext") return lang;
  return languageForExt(ext.value);
});

async function load() {
  const my = ++token;
  content.value = null;
  error.value = null;
  if (props.item.status === "missing") return;
  loading.value = true;
  try {
    const raw = await ipc.getArtifactContent(contentPath.value);
    if (my === token) content.value = normalizeContent(contentPath.value, raw);
  } catch (e) {
    if (my === token) error.value = errMsg(e);
  } finally {
    if (my === token) loading.value = false;
  }
}

watch(
  () => [contentPath.value, scan.snapshot] as const,
  () => void load(),
  { immediate: true },
);
watch(
  () => props.item.id,
  () => (showContent.value = !props.collapsedContent),
);

const kindLabel = computed(() =>
  props.item.kind ? (KIND_LABEL[props.item.kind as keyof typeof KIND_LABEL] ?? props.item.kind) : null,
);
const status = computed(() => props.item.status ?? "present");
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="shrink-0 space-y-2 border-b border-line px-5 py-3">
      <div class="flex flex-wrap items-center gap-2">
        <h2 class="min-w-0 break-words text-sm font-semibold">{{ item.name }}</h2>
        <Badge v-if="kindLabel">{{ kindLabel }}</Badge>
        <ScopeBadge :scope="item.scope" />
        <Badge :tone="STATUS_TONE[status]">{{ STATUS_LABEL[status] }}</Badge>
        <span class="ml-auto flex items-center gap-1 text-2xs text-subtle" title="This app never modifies Claude configuration">
          <Icon name="lock" :size="11" /> read-only
        </span>
      </div>
      <p v-if="item.subtitle" class="text-xs text-muted">{{ item.subtitle }}</p>
      <dl class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-0.5 text-xs">
        <dt class="text-subtle">Path</dt>
        <dd class="break-all font-mono text-[11.5px] text-muted" :title="item.path">
          {{ shortenPath(item.path) }}
        </dd>
        <template v-if="item.projectPath">
          <dt class="text-subtle">Project</dt>
          <dd class="break-all font-mono text-[11.5px] text-muted">
            {{ shortenPath(item.projectPath) }}
          </dd>
        </template>
        <dt class="text-subtle">Modified</dt>
        <dd class="text-muted">{{ formatTime(item.mtime) }}</dd>
      </dl>
    </div>

    <div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-5 py-4">
      <slot name="extra" />

      <section>
        <button
          v-if="collapsedContent"
          type="button"
          class="label mb-2 flex items-center gap-1 hover:text-fg"
          @click="showContent = !showContent"
        >
          <Icon
            name="chevron-right"
            :size="11"
            :class="showContent && 'rotate-90'"
            class="transition-transform"
          />
          {{ contentLabel }}
        </button>

        <template v-if="showContent">
          <LoadingBlock v-if="loading" :lines="6" />
          <EmptyState
            v-else-if="item.status === 'missing'"
            compact
            title="File not found"
            description="The file was discovered earlier but no longer exists on disk."
          />
          <EmptyState
            v-else-if="error"
            compact
            tone="danger"
            title="Could not load content"
            :description="error"
          />
          <template v-else-if="content">
            <MarkdownViewer v-if="mode === 'markdown'" :source="content.text" />
            <JsonViewer v-else-if="mode === 'json'" :text="content.text" />
            <CodeViewer v-else :code="content.text" :language="language" line-numbers />
          </template>
        </template>
      </section>
    </div>
  </div>
</template>
