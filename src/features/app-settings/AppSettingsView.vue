<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { LOCALES } from "@/i18n";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { useUpdatesStore } from "@/stores/updates";
import { ipc } from "@/lib/ipc";
import { shortenPath } from "@/lib/format";
import { TOOLING_PROVIDERS } from "@/lib/tooling";
import type { AppLocale, Preferences, ThemePreference, ToolingPresence } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import Icon from "@/components/common/Icon.vue";

const { t } = useI18n();
const prefs = usePreferencesStore();
const scan = useScanStore();
const updates = useUpdatesStore();

const THEMES = computed(() =>
  (["dark", "light", "system"] as ThemePreference[]).map((value) => ({
    value,
    label: t(`theme.${value}`),
  })),
);

const LOCALE_OPTIONS = computed(() =>
  LOCALES.map((value) => ({
    value,
    label: t(`locale.${value}`),
  })),
);

const presence = ref<ToolingPresence>({ claude: false, cursor: false, codex: false });

async function loadPresence() {
  try {
    presence.value = await ipc.getToolingPresence();
  } catch {
    /* backend may not expose the command yet */
  }
}

onMounted(() => {
  void loadPresence();
});

watch(
  () => prefs.loaded,
  (loaded) => {
    if (loaded) void loadPresence();
  },
);

function isLastEnabled(prefKey: (typeof TOOLING_PROVIDERS)[number]["prefKey"]): boolean {
  const p = prefs.prefs;
  const enabled = TOOLING_PROVIDERS.filter((t) => p[t.prefKey]);
  return enabled.length === 1 && enabled[0]!.prefKey === prefKey;
}

function onTrackToggle(
  prefKey: (typeof TOOLING_PROVIDERS)[number]["prefKey"],
  checked: boolean,
) {
  void prefs.update({ [prefKey]: checked } as Partial<Preferences>);
}

const statusText = computed(() => {
  switch (updates.status) {
    case "checking":
      return t("settings.updateChecking");
    case "up-to-date":
      return t("settings.updateUpToDate");
    case "update-available":
      return updates.latestVersion
        ? t("settings.updateAvailableVersion", { version: updates.latestVersion })
        : t("settings.updateAvailable");
    case "error":
      return updates.error
        ? t("settings.updateErrorWithDetail", { error: updates.error })
        : t("settings.updateError");
    default:
      return null;
  }
});

const statusTone = computed(() => {
  switch (updates.status) {
    case "up-to-date":
      return "text-ok";
    case "update-available":
      return "text-accent";
    case "error":
      return "text-warn";
    default:
      return "text-muted";
  }
});

const prefsBanner = computed(() => {
  if (prefs.toolingErrorKey) return t(prefs.toolingErrorKey);
  if (!prefs.error) return null;
  return t("settings.syncError", { error: prefs.error });
});

async function openRelease() {
  const url = updates.releaseUrl;
  if (!url || !/^https?:/i.test(url)) return;
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch {
    /* opener unavailable (browser dev) */
  }
}

function setLocale(locale: AppLocale) {
  void prefs.setLocale(locale);
}
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="mx-auto max-w-2xl space-y-6 px-6 py-5">
      <div
        v-if="prefsBanner"
        class="rounded-md border border-warn/30 bg-warn/10 px-3 py-1.5 text-xs text-warn"
      >
        {{ prefsBanner }}
      </div>

      <section>
        <h2 class="label mb-2">{{ t("settings.appearance") }}</h2>
        <div class="card divide-y divide-line">
          <div class="flex items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <div class="text-[13px] font-medium">{{ t("settings.theme") }}</div>
              <div class="text-xs text-muted">{{ t("settings.themeHint") }}</div>
            </div>
            <div class="flex items-center gap-0.5 rounded-md border border-line p-0.5">
              <button
                v-for="th in THEMES"
                :key="th.value"
                type="button"
                class="chip !h-6"
                :class="prefs.prefs.theme === th.value && 'chip-active'"
                @click="prefs.setTheme(th.value)"
              >
                {{ th.label }}
              </button>
            </div>
          </div>
          <div class="flex items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <div class="text-[13px] font-medium">{{ t("settings.language") }}</div>
              <div class="text-xs text-muted">{{ t("settings.languageHint") }}</div>
            </div>
            <div class="flex items-center gap-0.5 rounded-md border border-line p-0.5">
              <button
                v-for="opt in LOCALE_OPTIONS"
                :key="opt.value"
                type="button"
                class="chip !h-6"
                :class="prefs.prefs.locale === opt.value && 'chip-active'"
                @click="setLocale(opt.value)"
              >
                {{ opt.label }}
              </button>
            </div>
          </div>
        </div>
      </section>

      <section>
        <h2 class="label mb-2">{{ t("settings.tooling") }}</h2>
        <div class="card divide-y divide-line">
          <label
            v-for="provider in TOOLING_PROVIDERS"
            :key="provider.id"
            class="flex items-center justify-between gap-4 px-3 py-2.5"
            :class="
              isLastEnabled(provider.prefKey)
                ? 'cursor-not-allowed opacity-80'
                : 'cursor-pointer'
            "
          >
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <span class="text-[13px] font-medium">{{ provider.label }}</span>
                <Badge
                  :tone="presence[provider.id] ? 'ok' : 'neutral'"
                  class="!text-2xs"
                >
                  {{ presence[provider.id] ? t("settings.found") : t("settings.notFound") }}
                </Badge>
              </div>
              <div class="text-xs text-muted">
                {{ t("settings.toolingInclude", { label: provider.label }) }}
              </div>
            </div>
            <input
              type="checkbox"
              class="h-3.5 w-3.5 shrink-0 accent-[rgb(var(--accent))]"
              :checked="prefs.prefs[provider.prefKey]"
              :disabled="isLastEnabled(provider.prefKey)"
              @change="
                onTrackToggle(
                  provider.prefKey,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
          </label>
        </div>
        <p class="mt-1.5 text-2xs text-subtle">
          {{ t("settings.toolingAtLeastOne") }}
        </p>
      </section>

      <section>
        <h2 class="label mb-2">{{ t("settings.scanningSection") }}</h2>
        <div class="card divide-y divide-line">
          <label
            class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5"
          >
            <div>
              <div class="text-[13px] font-medium">
                {{ t("settings.watchOnStart") }}
              </div>
              <div class="text-xs text-muted">
                {{ t("settings.watchOnStartHint") }}
              </div>
            </div>
            <input
              type="checkbox"
              class="h-3.5 w-3.5 accent-[rgb(var(--accent))]"
              :checked="prefs.prefs.watchOnStart"
              @change="
                prefs.update({
                  watchOnStart: ($event.target as HTMLInputElement).checked,
                })
              "
            />
          </label>
          <label
            class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5"
          >
            <div>
              <div class="text-[13px] font-medium">{{ t("settings.notifications") }}</div>
              <div class="text-xs text-muted">
                {{ t("settings.notificationsHint") }}
              </div>
            </div>
            <input
              type="checkbox"
              class="h-3.5 w-3.5 accent-[rgb(var(--accent))]"
              :checked="prefs.prefs.notifications"
              @change="
                prefs.update({
                  notifications: ($event.target as HTMLInputElement).checked,
                })
              "
            />
          </label>
          <div class="px-3 py-2.5">
            <div class="mb-1 text-[13px] font-medium">{{ t("settings.scanRoots") }}</div>
            <ul v-if="scan.scanRoots.length" class="space-y-0.5">
              <li
                v-for="r in scan.scanRoots"
                :key="r.path"
                class="font-mono text-[11.5px] text-muted"
              >
                {{ shortenPath(r.path) }}
              </li>
            </ul>
            <p v-else class="text-xs text-muted">{{ t("settings.scanRootsEmpty") }}</p>
            <RouterLink
              to="/projects"
              class="mt-1.5 inline-block text-xs text-accent hover:underline"
            >
              {{ t("settings.manageOnProjects") }}
            </RouterLink>
          </div>
        </div>
      </section>

      <section>
        <h2 class="label mb-2">{{ t("settings.editing") }}</h2>
        <div class="card divide-y divide-line">
          <label
            class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5"
          >
            <div>
              <div class="text-[13px] font-medium">
                {{ t("settings.allowEditing") }}
              </div>
              <div class="text-xs text-muted">
                {{ t("settings.allowEditingHint") }}
                <br />
                <b>{{ t("settings.allowEditingSecrets") }}</b>
              </div>
            </div>
            <input
              type="checkbox"
              class="h-3.5 w-3.5 shrink-0 accent-[rgb(var(--accent))]"
              :checked="prefs.prefs.allowEditing"
              @change="
                prefs.update({
                  allowEditing: ($event.target as HTMLInputElement).checked,
                })
              "
            />
          </label>
        </div>
      </section>

      <section>
        <h2 class="label mb-2">{{ t("settings.about") }}</h2>
        <dl
          class="card grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1 px-3 py-2.5 text-xs"
        >
          <dt class="text-subtle">{{ t("settings.name") }}</dt>
          <dd>{{ scan.appInfo?.name ?? "Agent Scope" }}</dd>
          <dt class="text-subtle">{{ t("settings.version") }}</dt>
          <dd class="font-mono">{{ scan.appInfo?.version ?? "0.6.0" }}</dd>
          <template v-if="prefs.prefs.trackClaude && scan.globalMeta.configDir">
            <dt class="text-subtle">{{ t("settings.claudeDir") }}</dt>
            <dd class="break-all font-mono">
              {{ shortenPath(scan.globalMeta.configDir) }}
            </dd>
          </template>
          <template
            v-if="prefs.prefs.trackCursor && scan.globalMeta.cursorConfigDir"
          >
            <dt class="text-subtle">{{ t("settings.cursorDir") }}</dt>
            <dd class="break-all font-mono">
              {{ shortenPath(scan.globalMeta.cursorConfigDir) }}
            </dd>
          </template>
          <template
            v-if="prefs.prefs.trackCodex && scan.globalMeta.codexConfigDir"
          >
            <dt class="text-subtle">{{ t("settings.codexDir") }}</dt>
            <dd class="break-all font-mono">
              {{ shortenPath(scan.globalMeta.codexConfigDir) }}
            </dd>
          </template>
          <template v-if="scan.globalMeta.managedDir">
            <dt class="text-subtle">{{ t("settings.managedDir") }}</dt>
            <dd class="break-all font-mono">
              {{ shortenPath(scan.globalMeta.managedDir) }}
            </dd>
          </template>
          <dt class="text-subtle">{{ t("settings.backend") }}</dt>
          <dd>
            <Badge :tone="scan.backendAvailable ? 'ok' : 'danger'">
              {{
                scan.backendAvailable
                  ? t("settings.backendConnected")
                  : t("settings.backendUnavailable")
              }}
            </Badge>
          </dd>
        </dl>
        <div class="card mt-2 space-y-2 px-3 py-2.5">
          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="btn"
              :disabled="updates.status === 'checking'"
              @click="updates.check()"
            >
              <Icon name="refresh" :size="12" :class="updates.status === 'checking' && 'animate-spin'" />
              {{ t("settings.checkUpdates") }}
            </button>
            <button
              v-if="updates.updateAvailable && updates.releaseUrl"
              type="button"
              class="btn-ghost"
              @click="openRelease"
            >
              {{ t("settings.openRelease") }}
            </button>
          </div>
          <p v-if="statusText" class="text-xs" :class="statusTone">{{ statusText }}</p>
          <p
            v-if="updates.updateAvailable && scan.appInfo?.platform === 'macos'"
            class="text-2xs text-subtle"
          >
            {{ t("settings.brewHint") }}
            <code class="font-mono text-muted">brew upgrade --cask agent-scope</code>
          </p>
          <p
            v-else-if="
              updates.updateAvailable &&
              (scan.appInfo?.platform === 'windows' || scan.appInfo?.platform === 'linux')
            "
            class="text-2xs text-subtle"
          >
            {{ t("settings.installerHint") }}
          </p>
        </div>
        <p class="mt-2 flex items-center gap-1.5 text-2xs text-subtle">
          <Icon name="lock" :size="11" />
          {{ t("settings.defaultViewHint") }}
        </p>
      </section>
    </div>
  </div>
</template>
