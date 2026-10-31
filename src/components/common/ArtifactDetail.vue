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
import { usePreferencesStore } from "@/stores/preferences";
import Badge from "./Badge.vue";
import ProductBadge from "./ProductBadge.vue";
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
  { collapsedContent: false, contentLabel: "Содержимое файла" },
);

const scan = useScanStore();
const prefs = usePreferencesStore();
const content = ref<ViewContent | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const showContent = ref(!props.collapsedContent);
let token = 0;

const editing = ref(false);
const editLoading = ref(false);
const saving = ref(false);
const draft = ref("");
const editBaseline = ref("");
const contentHash = ref<string | null>(null);
const editError = ref<string | null>(null);
const undoPrevious = ref<string | null>(null);
const undoHash = ref<string | null>(null);

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

const allowEditing = computed(() => prefs.prefs.allowEditing);
const canEdit = computed(
  () => allowEditing.value && props.item.status !== "missing" && !!contentPath.value,
);
const dirty = computed(() => editing.value && draft.value !== editBaseline.value);

function localizeEditError(e: unknown): string {
  const msg = errMsg(e);
  const lower = msg.toLowerCase();
  if (lower.includes("conflict") || lower.includes("changed on disk")) {
    return "Файл изменился на диске. Перезагрузите и сохраните снова.";
  }
  if (lower.includes("editing is disabled") || lower.includes("allowediting")) {
    return "Редактирование выключено в настройках приложения.";
  }
  if (lower.includes("json") || lower.includes("expected") || lower.includes("eof")) {
    return `Некорректный JSON: ${msg}`;
  }
  if (lower.includes("access denied") || lower.includes("not writable") || lower.includes("credentials")) {
    return `Запись запрещена: ${msg}`;
  }
  return msg;
}

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

function confirmLeaveDirty(): boolean {
  if (!dirty.value) return true;
  return window.confirm("Есть несохранённые изменения. Уйти без сохранения?");
}

async function startEdit() {
  if (!canEdit.value) return;
  editError.value = null;
  editLoading.value = true;
  try {
    const raw = await ipc.getArtifactContentForEdit(contentPath.value);
    draft.value = raw.text;
    editBaseline.value = raw.text;
    contentHash.value = raw.contentHash;
    undoPrevious.value = null;
    undoHash.value = null;
    editing.value = true;
  } catch (e) {
    editError.value = localizeEditError(e);
  } finally {
    editLoading.value = false;
  }
}

function resetEditState() {
  editing.value = false;
  draft.value = "";
  editBaseline.value = "";
  contentHash.value = null;
  editError.value = null;
}

function cancelEdit() {
  if (!confirmLeaveDirty()) return;
  resetEditState();
}

async function saveEdit() {
  if (!contentHash.value) return;
  editError.value = null;
  saving.value = true;
  try {
    const result = await ipc.saveArtifactContent(
      contentPath.value,
      draft.value,
      contentHash.value,
    );
    undoPrevious.value = result.previousContent;
    undoHash.value = result.contentHash;
    resetEditState();
    await scan.refresh({ silent: true });
    await load();
  } catch (e) {
    editError.value = localizeEditError(e);
  } finally {
    saving.value = false;
  }
}

async function undoLastSave() {
  if (undoPrevious.value == null || !undoHash.value) return;
  editError.value = null;
  saving.value = true;
  try {
    await ipc.saveArtifactContent(contentPath.value, undoPrevious.value, undoHash.value);
    undoPrevious.value = null;
    undoHash.value = null;
    await scan.refresh({ silent: true });
    await load();
  } catch (e) {
    editError.value = localizeEditError(e);
  } finally {
    saving.value = false;
  }
}

function basenameOf(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

async function hashForMutation(): Promise<string> {
  const raw = await ipc.getArtifactContentForEdit(contentPath.value);
  return raw.contentHash;
}

async function renameFile() {
  if (!canEdit.value || editing.value) return;
  if (!confirmLeaveDirty()) return;
  const current = basenameOf(contentPath.value);
  const next = window.prompt("Новое имя файла (только имя в этой папке):", current);
  if (next == null) return;
  const trimmed = next.trim();
  if (!trimmed || trimmed === current) return;
  editError.value = null;
  saving.value = true;
  try {
    const hash = await hashForMutation();
    await ipc.renameArtifact(contentPath.value, trimmed, hash);
    undoPrevious.value = null;
    undoHash.value = null;
    resetEditState();
    await scan.refresh({ silent: true });
  } catch (e) {
    editError.value = localizeEditError(e);
  } finally {
    saving.value = false;
  }
}

async function deleteFile() {
  if (!canEdit.value || editing.value) return;
  if (!confirmLeaveDirty()) return;
  const ok = window.confirm(
    `Удалить файл навсегда?\n\n${contentPath.value}\n\nЭто действие нельзя отменить из приложения.`,
  );
  if (!ok) return;
  editError.value = null;
  saving.value = true;
  try {
    const hash = await hashForMutation();
    await ipc.deleteArtifact(contentPath.value, hash);
    undoPrevious.value = null;
    undoHash.value = null;
    resetEditState();
    await scan.refresh({ silent: true });
  } catch (e) {
    editError.value = localizeEditError(e);
  } finally {
    saving.value = false;
  }
}

watch(
  () => contentPath.value,
  () => {
    if (editing.value && !confirmLeaveDirty()) return;
    resetEditState();
    undoPrevious.value = null;
    undoHash.value = null;
    void load();
  },
  { immediate: true },
);

watch(
  () => scan.snapshot,
  () => {
    if (editing.value) return;
    void load();
  },
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
        <ProductBadge :product="item.product" />
        <ScopeBadge :scope="item.scope" />
        <Badge :tone="STATUS_TONE[status]">{{ STATUS_LABEL[status] }}</Badge>
        <span
          v-if="!allowEditing"
          class="ml-auto flex items-center gap-1 text-2xs text-subtle"
          title="По умолчанию только просмотр. Запись существующих файлов из allowlist включается в настройках приложения."
        >
          <Icon name="lock" :size="11" /> только просмотр
        </span>
        <div v-else class="ml-auto flex items-center gap-2">
          <span
            v-if="!editing"
            class="flex items-center gap-1 text-2xs text-subtle"
            title="Редактирование включено. Сохраняется только сырой файл из allowlist."
          >
            можно редактировать
          </span>
          <template v-if="canEdit && !editing">
            <button
              type="button"
              class="chip !h-6"
              :disabled="editLoading || loading || saving"
              @click="startEdit"
            >
              {{ editLoading ? "Загрузка…" : "Изменить" }}
            </button>
            <button
              type="button"
              class="chip !h-6"
              :disabled="loading || saving"
              @click="renameFile"
            >
              Переименовать
            </button>
            <button
              type="button"
              class="chip !h-6 text-danger"
              :disabled="loading || saving"
              @click="deleteFile"
            >
              Удалить
            </button>
          </template>
          <template v-if="editing">
            <button
              type="button"
              class="chip !h-6"
              :disabled="saving"
              @click="cancelEdit"
            >
              Отмена
            </button>
            <button
              type="button"
              class="chip chip-active !h-6"
              :disabled="saving || !dirty"
              @click="saveEdit"
            >
              {{ saving ? "Сохранение…" : "Сохранить" }}
            </button>
          </template>
          <button
            v-if="!editing && undoPrevious != null"
            type="button"
            class="chip !h-6"
            :disabled="saving"
            @click="undoLastSave"
          >
            Отменить сохранение
          </button>
        </div>
      </div>
      <p v-if="item.subtitle" class="text-xs text-muted">{{ item.subtitle }}</p>
      <dl class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-0.5 text-xs">
        <dt class="text-subtle">Путь</dt>
        <dd class="break-all font-mono text-[11.5px] text-muted" :title="item.path">
          {{ shortenPath(item.path) }}
        </dd>
        <template v-if="item.projectPath">
          <dt class="text-subtle">Проект</dt>
          <dd class="break-all font-mono text-[11.5px] text-muted">
            {{ shortenPath(item.projectPath) }}
          </dd>
        </template>
        <dt class="text-subtle">Изменён</dt>
        <dd class="text-muted">{{ formatTime(item.mtime) }}</dd>
      </dl>
    </div>

    <div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-5 py-4">
      <slot name="extra" />

      <div
        v-if="editError"
        class="rounded-md border border-danger/30 bg-danger/10 px-3 py-1.5 text-xs text-danger"
      >
        {{ editError }}
      </div>

      <section>
        <button
          v-if="collapsedContent && !editing"
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

        <template v-if="editing">
          <div
            class="mb-2 rounded-md border border-warn/30 bg-warn/10 px-3 py-1.5 text-xs text-warn"
          >
            В режиме редактирования секреты не маскируются. Не сохраняйте содержимое из режима просмотра
            с масками <span class="font-mono">***</span>.
          </div>
          <textarea
            v-model="draft"
            spellcheck="false"
            class="min-h-[16rem] w-full resize-y rounded-md border border-line bg-bg px-3 py-2 font-mono text-[12px] leading-relaxed text-fg outline-none focus:border-accent"
          />
        </template>

        <template v-else-if="showContent">
          <LoadingBlock v-if="loading" :lines="6" />
          <EmptyState
            v-else-if="item.status === 'missing'"
            compact
            title="Файл не найден"
            description="Файл был обнаружен ранее, но больше не существует на диске."
          />
          <EmptyState
            v-else-if="error"
            compact
            tone="danger"
            title="Не удалось загрузить содержимое"
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
