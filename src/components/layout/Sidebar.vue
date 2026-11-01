<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { RouterLink, useRoute } from "vue-router";
import { useScanStore } from "@/stores/scan";
import { useUpdatesStore } from "@/stores/updates";
import { relativeTime } from "@/lib/format";
import Icon, { type IconName } from "@/components/common/Icon.vue";
import StatusDot from "@/components/common/StatusDot.vue";

interface NavItem {
  to: string;
  labelKey: string;
  icon: IconName;
  count?: number;
  /** Route names/prefixes that keep this item highlighted. */
  match?: string;
}

const { t } = useI18n();
const scan = useScanStore();
const updates = useUpdatesStore();
const route = useRoute();

const main = computed<NavItem[]>(() => {
  const d = scan.dashboard;
  return [
    { to: "/", labelKey: "nav.overview", icon: "dashboard" },
    { to: "/global", labelKey: "nav.global", icon: "globe" },
    {
      to: "/projects",
      labelKey: "nav.projects",
      icon: "folder",
      count: d.projectCount,
      match: "/projects",
    },
    { to: "/skills", labelKey: "nav.skills", icon: "zap", count: scan.skillEntries.length },
    { to: "/rules", labelKey: "nav.rules", icon: "file-text", count: d.rules },
    { to: "/agents", labelKey: "nav.agents", icon: "bot", count: d.agents },
    { to: "/hooks", labelKey: "nav.hooks", icon: "link", count: d.hooks },
    { to: "/mcp", labelKey: "nav.mcp", icon: "server", count: d.mcpServers },
    { to: "/plugins", labelKey: "nav.plugins", icon: "box", count: d.plugins },
    { to: "/permissions", labelKey: "nav.permissions", icon: "shield" },
    { to: "/settings", labelKey: "nav.settings", icon: "sliders" },
    { to: "/memory", labelKey: "nav.memory", icon: "database" },
  ];
});

function isActive(item: NavItem): boolean {
  if (item.to === "/") return route.path === "/";
  return route.path === item.to || route.path.startsWith((item.match ?? item.to) + "/");
}

const status = computed(() => {
  if (scan.scanning) return { tone: "accent" as const, pulse: true, text: t("status.scanning") };
  if (scan.error && !scan.backendAvailable)
    return { tone: "danger" as const, pulse: false, text: t("status.backendUnavailable") };
  if (scan.error)
    return { tone: "warn" as const, pulse: false, text: t("status.lastScanFailed") };
  if (!scan.initialized)
    return { tone: "neutral" as const, pulse: true, text: t("status.starting") };
  return {
    tone: "ok" as const,
    pulse: false,
    text: t("status.scannedAt", { time: relativeTime(scan.lastScanAt) }),
  };
});
</script>

<template>
  <aside class="flex h-full w-[212px] shrink-0 flex-col border-r border-line bg-surface">
    <div class="flex h-11 shrink-0 items-center gap-2 px-3" data-tauri-drag-region>
      <div
        class="flex h-5 w-5 items-center justify-center rounded bg-accent/15 text-accent"
        aria-hidden="true"
      >
        <Icon name="eye" :size="13" />
      </div>
      <div class="min-w-0">
        <div class="text-[13px] font-semibold tracking-tight">Agent Scope</div>
      </div>
    </div>

    <nav class="flex-1 space-y-px overflow-y-auto px-2 pb-2" :aria-label="t('nav.sectionsAria')">
      <RouterLink
        v-for="item in main"
        :key="item.to"
        :to="item.to"
        class="group flex h-7 items-center gap-2 rounded-md px-2 text-[13px] transition-colors"
        :class="
          isActive(item)
            ? 'bg-elevated text-fg'
            : 'text-muted hover:bg-elevated/60 hover:text-fg'
        "
      >
        <Icon
          :name="item.icon"
          :size="15"
          :class="isActive(item) ? 'text-accent' : 'text-subtle group-hover:text-muted'"
        />
        <span class="flex-1 truncate">{{ t(item.labelKey) }}</span>
        <span
          v-if="item.count"
          class="font-mono text-2xs tabular-nums"
          :class="isActive(item) ? 'text-muted' : 'text-subtle'"
        >
          {{ item.count }}
        </span>
      </RouterLink>
    </nav>

    <div class="shrink-0 space-y-px border-t border-line px-2 py-2">
      <RouterLink
        to="/app-settings"
        class="flex h-7 items-center gap-2 rounded-md px-2 text-[13px] transition-colors"
        :class="
          route.path === '/app-settings'
            ? 'bg-elevated text-fg'
            : 'text-muted hover:bg-elevated/60 hover:text-fg'
        "
      >
        <Icon name="settings" :size="15" class="text-subtle" />
        {{ t("nav.appSettings") }}
      </RouterLink>
      <RouterLink
        v-if="updates.updateAvailable && updates.latestVersion"
        to="/app-settings"
        class="mx-0.5 flex h-6 items-center gap-1.5 truncate rounded-md bg-accent/10 px-2 text-2xs text-accent transition-colors hover:bg-accent/15"
        :title="t('settings.updateAvailableVersion', { version: updates.latestVersion })"
      >
        <Icon name="inbox" :size="11" class="shrink-0" />
        <span class="truncate"
          >{{ t("settings.updateAvailable") }} {{ updates.latestVersion }}</span
        >
      </RouterLink>
      <div class="flex items-center justify-between gap-2 px-2 pt-1.5 text-2xs text-subtle">
        <span class="flex min-w-0 items-center gap-1.5" :title="scan.error ?? undefined">
          <StatusDot :tone="status.tone" :pulse="status.pulse" />
          <span class="truncate">{{ status.text }}</span>
        </span>
        <RouterLink
          to="/app-settings"
          class="font-mono hover:text-fg"
          :title="t('nav.appSettings')"
        >
          v{{ scan.appInfo?.version ?? "0.6.1" }}
        </RouterLink>
      </div>
    </div>
  </aside>
</template>
