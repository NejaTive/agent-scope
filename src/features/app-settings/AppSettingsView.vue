<script setup lang="ts">
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { shortenPath } from "@/lib/format";
import type { ThemePreference } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import Icon from "@/components/common/Icon.vue";

const prefs = usePreferencesStore();
const scan = useScanStore();

const THEMES: Array<{ value: ThemePreference; label: string }> = [
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
  { value: "system", label: "System" },
];
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="mx-auto max-w-2xl space-y-6 px-6 py-5">
      <div
        v-if="prefs.error"
        class="rounded-md border border-warn/30 bg-warn/10 px-3 py-1.5 text-xs text-warn"
      >
        Preferences could not be synced with the backend ({{ prefs.error }}). Changes are kept locally.
      </div>

      <section>
        <h2 class="label mb-2">Appearance</h2>
        <div class="card flex items-center justify-between gap-4 px-3 py-2.5">
          <div>
            <div class="text-[13px] font-medium">Theme</div>
            <div class="text-xs text-muted">Dark is the default. System follows your OS setting.</div>
          </div>
          <div class="flex items-center gap-0.5 rounded-md border border-line p-0.5">
            <button
              v-for="t in THEMES"
              :key="t.value"
              type="button"
              class="chip !h-6"
              :class="prefs.prefs.theme === t.value && 'chip-active'"
              @click="prefs.setTheme(t.value)"
            >
              {{ t.label }}
            </button>
          </div>
        </div>
      </section>

      <section>
        <h2 class="label mb-2">Scanning</h2>
        <div class="card divide-y divide-line">
          <label class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <div class="text-[13px] font-medium">Watch for file changes on start</div>
              <div class="text-xs text-muted">
                Soft-refresh automatically when Claude config files change on disk. Toggling this starts or stops
                the watcher immediately.
              </div>
            </div>
            <input
              type="checkbox"
              class="h-3.5 w-3.5 accent-[rgb(var(--accent))]"
              :checked="prefs.prefs.watchOnStart"
              @change="prefs.update({ watchOnStart: ($event.target as HTMLInputElement).checked })"
            />
          </label>
          <label class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <div class="text-[13px] font-medium">Notifications</div>
              <div class="text-xs text-muted">Notify when Claude config files change on disk.</div>
            </div>
            <input
              type="checkbox"
              class="h-3.5 w-3.5 accent-[rgb(var(--accent))]"
              :checked="prefs.prefs.notifications"
              @change="prefs.update({ notifications: ($event.target as HTMLInputElement).checked })"
            />
          </label>
          <div class="px-3 py-2.5">
            <div class="mb-1 text-[13px] font-medium">Scan roots</div>
            <ul v-if="scan.scanRoots.length" class="space-y-0.5">
              <li v-for="r in scan.scanRoots" :key="r.path" class="font-mono text-[11.5px] text-muted">
                {{ shortenPath(r.path) }}
              </li>
            </ul>
            <p v-else class="text-xs text-muted">None configured.</p>
            <RouterLink to="/projects" class="mt-1.5 inline-block text-xs text-accent hover:underline">
              Manage on the Projects page →
            </RouterLink>
          </div>
        </div>
      </section>

      <section>
        <h2 class="label mb-2">About</h2>
        <dl class="card grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1 px-3 py-2.5 text-xs">
          <dt class="text-subtle">Name</dt>
          <dd>{{ scan.appInfo?.name ?? "Agent Scope" }}</dd>
          <dt class="text-subtle">Version</dt>
          <dd class="font-mono">{{ scan.appInfo?.version ?? "0.1.0" }}</dd>
          <template v-if="scan.globalMeta.configDir">
            <dt class="text-subtle">Claude home</dt>
            <dd class="break-all font-mono">{{ shortenPath(scan.globalMeta.configDir) }}</dd>
          </template>
          <template v-if="scan.globalMeta.managedDir">
            <dt class="text-subtle">Managed dir</dt>
            <dd class="break-all font-mono">{{ shortenPath(scan.globalMeta.managedDir) }}</dd>
          </template>
          <dt class="text-subtle">Backend</dt>
          <dd>
            <Badge :tone="scan.backendAvailable ? 'ok' : 'danger'">
              {{ scan.backendAvailable ? "connected" : "unavailable" }}
            </Badge>
          </dd>
        </dl>
        <p class="mt-2 flex items-center gap-1.5 text-2xs text-subtle">
          <Icon name="lock" :size="11" /> Agent Scope is read-only: it never creates, edits or deletes Claude configuration.
        </p>
      </section>
    </div>
  </div>
</template>
