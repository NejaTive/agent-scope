<script setup lang="ts">
import { computed } from "vue";
import { useScanStore } from "@/stores/scan";
import type { BrowserItem } from "@/lib/browser";
import ArtifactBrowser from "@/components/common/ArtifactBrowser.vue";
import FieldList from "@/components/common/FieldList.vue";

const scan = useScanStore();
const byId = computed(() => new Map(scan.rules.map((r) => [r.id, r])));

const items = computed<BrowserItem[]>(() =>
  scan.rules.map((r) => ({
    id: r.id,
    name: r.name,
    subtitle: r.paths.length ? `Applies to ${r.paths.join(", ")}` : null,
    scope: r.scope,
    status: r.status,
    mtime: r.mtime,
    path: r.path,
    projectPath: r.projectPath,
    artifactId: r.artifactId,
    kind: "rule",
    searchText: (r.paths ?? []).join(" "),
  })),
);
</script>

<template>
  <ArtifactBrowser
    :items="items"
    :loading="!scan.initialized"
    empty-title="No rules found"
    empty-description="Rules live in .claude/rules/*.md at user or project level."
  >
    <template #detail="{ item }">
      <FieldList
        title="Scope"
        :fields="[
          { label: 'Path globs', value: byId.get(item.id)?.paths },
          { label: 'Conditional', value: byId.get(item.id)?.conditional ? 'yes (applies only to matching paths)' : null },
        ]"
      />
    </template>
  </ArtifactBrowser>
</template>
