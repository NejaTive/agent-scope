<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { useUpdatesStore } from "@/stores/updates";
import { ipc } from "@/lib/ipc";
import { shortenPath } from "@/lib/format";
import { TOOLING_PROVIDERS } from "@/lib/tooling";
import type { Preferences, ThemePreference, ToolingPresence } from "@/types/config";
import Badge from "@/components/common/Badge.vue";
import Icon from "@/components/common/Icon.vue";

const prefs = usePreferencesStore();
const scan = useScanStore();
const updates = useUpdatesStore();

const THEMES: Array<{ value: ThemePreference; label: string }> = [
  { value: "dark", label: "Тёмная" },
  { value: "light", label: "Светлая" },
  { value: "system", label: "Системная" },
];

const presence = ref<ToolingPresence>({ claude: false, cursor: false });

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
      return "Проверка…";
    case "up-to-date":
      return "Установлена актуальная версия";
    case "update-available":
      return updates.latestVersion
        ? `Доступна версия ${updates.latestVersion}`
        : "Доступна новая версия";
    case "error":
      return updates.error
        ? `Не удалось проверить обновления: ${updates.error}`
        : "Не удалось проверить обновления";
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
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="mx-auto max-w-2xl space-y-6 px-6 py-5">
      <div
        v-if="prefs.error"
        class="rounded-md border border-warn/30 bg-warn/10 px-3 py-1.5 text-xs text-warn"
      >
        {{
          prefs.error.includes("AI-инструмент")
            ? prefs.error
            : `Не удалось синхронизировать настройки с бэкендом (${prefs.error}). Изменения сохранены локально.`
        }}
      </div>

      <section>
        <h2 class="label mb-2">Внешний вид</h2>
        <div class="card flex items-center justify-between gap-4 px-3 py-2.5">
          <div>
            <div class="text-[13px] font-medium">Тема</div>
            <div class="text-xs text-muted">
              По умолчанию — тёмная. Системная следует за настройкой ОС.
            </div>
          </div>
          <div
            class="flex items-center gap-0.5 rounded-md border border-line p-0.5"
          >
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
        <h2 class="label mb-2">AI-инструменты</h2>
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
                  {{ presence[provider.id] ? "обнаружен" : "не найден" }}
                </Badge>
              </div>
              <div class="text-xs text-muted">
                Включать {{ provider.label }} в сканирование и отслеживание
                изменений.
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
          Хотя бы один инструмент должен оставаться включённым.
        </p>
      </section>

      <section>
        <h2 class="label mb-2">Сканирование</h2>
        <div class="card divide-y divide-line">
          <label
            class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5"
          >
            <div>
              <div class="text-[13px] font-medium">
                Отслеживать изменения файлов при запуске
              </div>
              <div class="text-xs text-muted">
                Автоматическое мягкое обновление при изменении конфигурации
                включённых AI-инструментов на диске. Переключение сразу
                запускает или останавливает наблюдатель.
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
              <div class="text-[13px] font-medium">Уведомления</div>
              <div class="text-xs text-muted">
                Уведомлять об изменении конфигурации включённых AI-инструментов
                на диске.
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
            <div class="mb-1 text-[13px] font-medium">Корни сканирования</div>
            <ul v-if="scan.scanRoots.length" class="space-y-0.5">
              <li
                v-for="r in scan.scanRoots"
                :key="r.path"
                class="font-mono text-[11.5px] text-muted"
              >
                {{ shortenPath(r.path) }}
              </li>
            </ul>
            <p v-else class="text-xs text-muted">Не заданы.</p>
            <RouterLink
              to="/projects"
              class="mt-1.5 inline-block text-xs text-accent hover:underline"
            >
              Управление на странице «Проекты» →
            </RouterLink>
          </div>
        </div>
      </section>

      <section>
        <h2 class="label mb-2">Редактирование</h2>
        <div class="card divide-y divide-line">
          <label
            class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5"
          >
            <div>
              <div class="text-[13px] font-medium">
                Разрешить редактирование файлов конфигурации
              </div>
              <div class="text-xs text-muted">
                Включает редактирование, переименование и удаление существующих файлов.
                <br />
                <b> В режиме правки секреты не маскируются.</b>
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
        <h2 class="label mb-2">О приложении</h2>
        <dl
          class="card grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1 px-3 py-2.5 text-xs"
        >
          <dt class="text-subtle">Название</dt>
          <dd>{{ scan.appInfo?.name ?? "Agent Scope" }}</dd>
          <dt class="text-subtle">Версия</dt>
          <dd class="font-mono">{{ scan.appInfo?.version ?? "0.4.0" }}</dd>
          <template v-if="prefs.prefs.trackClaude && scan.globalMeta.configDir">
            <dt class="text-subtle">Каталог Claude Code</dt>
            <dd class="break-all font-mono">
              {{ shortenPath(scan.globalMeta.configDir) }}
            </dd>
          </template>
          <template
            v-if="prefs.prefs.trackCursor && scan.globalMeta.cursorConfigDir"
          >
            <dt class="text-subtle">Каталог Cursor</dt>
            <dd class="break-all font-mono">
              {{ shortenPath(scan.globalMeta.cursorConfigDir) }}
            </dd>
          </template>
          <template v-if="scan.globalMeta.managedDir">
            <dt class="text-subtle">Управляемый каталог</dt>
            <dd class="break-all font-mono">
              {{ shortenPath(scan.globalMeta.managedDir) }}
            </dd>
          </template>
          <dt class="text-subtle">Бэкенд</dt>
          <dd>
            <Badge :tone="scan.backendAvailable ? 'ok' : 'danger'">
              {{ scan.backendAvailable ? "подключён" : "недоступен" }}
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
              Проверить обновления
            </button>
            <button
              v-if="updates.updateAvailable && updates.releaseUrl"
              type="button"
              class="btn-ghost"
              @click="openRelease"
            >
              Открыть релиз
            </button>
          </div>
          <p v-if="statusText" class="text-xs" :class="statusTone">{{ statusText }}</p>
          <p
            v-if="updates.updateAvailable && scan.appInfo?.platform === 'macos'"
            class="text-2xs text-subtle"
          >
            Через Homebrew:
            <code class="font-mono text-muted">brew upgrade --cask agent-scope</code>
          </p>
          <p
            v-else-if="
              updates.updateAvailable &&
              (scan.appInfo?.platform === 'windows' || scan.appInfo?.platform === 'linux')
            "
            class="text-2xs text-subtle"
          >
            Скачай новый установщик с страницы релиза.
          </p>
        </div>
        <p class="mt-2 flex items-center gap-1.5 text-2xs text-subtle">
          <Icon name="lock" :size="11" />
          По умолчанию — просмотр. Запись / rename / delete — opt-in и только
          для существующих файлов из allowlist; credentials и пути вне списка не
          трогаются.
        </p>
      </section>
    </div>
  </div>
</template>
