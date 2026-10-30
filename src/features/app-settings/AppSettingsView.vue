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
  { value: "dark", label: "Тёмная" },
  { value: "light", label: "Светлая" },
  { value: "system", label: "Системная" },
];
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="mx-auto max-w-2xl space-y-6 px-6 py-5">
      <div
        v-if="prefs.error"
        class="rounded-md border border-warn/30 bg-warn/10 px-3 py-1.5 text-xs text-warn"
      >
        Не удалось синхронизировать настройки с бэкендом ({{ prefs.error }}). Изменения сохранены локально.
      </div>

      <section>
        <h2 class="label mb-2">Внешний вид</h2>
        <div class="card flex items-center justify-between gap-4 px-3 py-2.5">
          <div>
            <div class="text-[13px] font-medium">Тема</div>
            <div class="text-xs text-muted">По умолчанию — тёмная. Системная следует за настройкой ОС.</div>
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
        <h2 class="label mb-2">Сканирование</h2>
        <div class="card divide-y divide-line">
          <label class="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <div class="text-[13px] font-medium">Отслеживать изменения файлов при запуске</div>
              <div class="text-xs text-muted">
                Автоматическое мягкое обновление при изменении конфигурации Claude на диске. Переключение сразу запускает или останавливает
                наблюдатель.
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
              <div class="text-[13px] font-medium">Уведомления</div>
              <div class="text-xs text-muted">Уведомлять об изменении конфигурации Claude на диске.</div>
            </div>
            <input
              type="checkbox"
              class="h-3.5 w-3.5 accent-[rgb(var(--accent))]"
              :checked="prefs.prefs.notifications"
              @change="prefs.update({ notifications: ($event.target as HTMLInputElement).checked })"
            />
          </label>
          <div class="px-3 py-2.5">
            <div class="mb-1 text-[13px] font-medium">Корни сканирования</div>
            <ul v-if="scan.scanRoots.length" class="space-y-0.5">
              <li v-for="r in scan.scanRoots" :key="r.path" class="font-mono text-[11.5px] text-muted">
                {{ shortenPath(r.path) }}
              </li>
            </ul>
            <p v-else class="text-xs text-muted">Не заданы.</p>
            <RouterLink to="/projects" class="mt-1.5 inline-block text-xs text-accent hover:underline">
              Управление на странице «Проекты» →
            </RouterLink>
          </div>
        </div>
      </section>

      <section>
        <h2 class="label mb-2">О приложении</h2>
        <dl class="card grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1 px-3 py-2.5 text-xs">
          <dt class="text-subtle">Название</dt>
          <dd>{{ scan.appInfo?.name ?? "Agent Scope" }}</dd>
          <dt class="text-subtle">Версия</dt>
          <dd class="font-mono">{{ scan.appInfo?.version ?? "0.1.0" }}</dd>
          <template v-if="scan.globalMeta.configDir">
            <dt class="text-subtle">Каталог Claude</dt>
            <dd class="break-all font-mono">{{ shortenPath(scan.globalMeta.configDir) }}</dd>
          </template>
          <template v-if="scan.globalMeta.managedDir">
            <dt class="text-subtle">Управляемый каталог</dt>
            <dd class="break-all font-mono">{{ shortenPath(scan.globalMeta.managedDir) }}</dd>
          </template>
          <dt class="text-subtle">Бэкенд</dt>
          <dd>
            <Badge :tone="scan.backendAvailable ? 'ok' : 'danger'">
              {{ scan.backendAvailable ? "подключён" : "недоступен" }}
            </Badge>
          </dd>
        </dl>
        <p class="mt-2 flex items-center gap-1.5 text-2xs text-subtle">
          <Icon name="lock" :size="11" /> Agent Scope только для чтения: не создаёт, не изменяет и не удаляет конфигурацию Claude.
        </p>
      </section>
    </div>
  </div>
</template>
