<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import type { BrowserItem } from "@/lib/browser";
import { trackedPathHint } from "@/lib/tooling";
import type { SkillEntry, UiSkill } from "@/types/config";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const { t } = useI18n();
const scan = useScanStore();
const prefs = usePreferencesStore();
const ui = useUiStore();
const tab = ref<"skill" | "command">("skill");

// If a global-search hit targets a command, open the Commands tab.
if (ui.focusArtifactId && scan.commands.some((c) => c.id === ui.focusArtifactId || c.artifactId === ui.focusArtifactId)) {
  tab.value = "command";
}

const entries = computed(() => (tab.value === "skill" ? scan.skills : scan.commands));
const byId = computed(() => new Map(entries.value.map((e) => [e.id, e])));

/** Read a (string-ish) field from the entry's frontmatter. */
function fm(e: SkillEntry, key: string): string | string[] | null {
  const f = e.frontmatter;
  if (!f || typeof f !== "object") return null;
  const v = (f as Record<string, unknown>)[key];
  if (typeof v === "string") return v;
  if (Array.isArray(v)) return v.map(String);
  return null;
}

function fmOf(id: string, key: string) {
  const e = byId.value.get(id);
  return e ? fm(e, key) : null;
}

const items = computed<BrowserItem[]>(() =>
  entries.value.map((e: UiSkill) => ({
    id: e.id,
    name: tab.value === "command" && !e.name.startsWith("/") ? `/${e.name}` : e.name,
    subtitle: e.description ?? null,
    scope: e.scope,
    status: e.status,
    mtime: e.mtime,
    path: e.path,
    projectPath: e.projectPath,
    artifactId: e.artifactId,
    kind: e.kind,
    product: e.product,
    searchText: [fm(e, "argument-hint"), fm(e, "model"), fm(e, "allowed-tools"), ...e.files]
      .filter(Boolean)
      .join(" "),
  })),
);

const emptyDescription = computed(() => {
  const skills = trackedPathHint(prefs.prefs, "skills/<name>/SKILL.md");
  const cmd = prefs.prefs.trackClaude ? t("skills.emptyCommandsSuffix") : "";
  return t("skills.emptyHint", { skills, commands: cmd });
});

function detailFields(id: string) {
  return [
    { label: t("skills.argumentHint"), value: fmOf(id, "argument-hint") },
    { label: t("skills.model"), value: fmOf(id, "model") },
    { label: t("skills.allowedTools"), value: fmOf(id, "allowed-tools") },
    { label: t("skills.files"), value: byId.value.get(id)?.files },
  ];
}
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    :empty-title="tab === 'skill' ? t('empty.noSkills') : t('skills.emptyCommands')"
    :empty-description="emptyDescription"
  >
    <template #header>
      <div class="flex shrink-0 items-center gap-0.5 border-b border-line px-2 py-1.5" role="tablist">
        <button
          type="button"
          role="tab"
          class="chip"
          :class="tab === 'skill' && 'chip-active'"
          :aria-selected="tab === 'skill'"
          @click="tab = 'skill'"
        >
          {{ t("skills.skills") }} <span class="font-mono text-2xs text-subtle">{{ scan.skills.length }}</span>
        </button>
        <button
          type="button"
          role="tab"
          class="chip"
          :class="tab === 'command' && 'chip-active'"
          :aria-selected="tab === 'command'"
          @click="tab = 'command'"
        >
          {{ t("skills.commands") }} <span class="font-mono text-2xs text-subtle">{{ scan.commands.length }}</span>
        </button>
      </div>
    </template>
    <template #detail="{ item }">
      <FieldList
        v-if="byId.get(item.id)"
        :title="t('skills.frontmatter')"
        :fields="detailFields(item.id)"
      />
    </template>
  </ArtifactBrowser>
</template>
