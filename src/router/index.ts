import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

declare module "vue-router" {
  interface RouteMeta {
    /** i18n key under `nav.*` (or full key path). */
    titleKey?: string;
    /** Whether the header search box filters this view. */
    searchable?: boolean;
  }
}

export const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "overview",
    component: () => import("@/features/dashboard/DashboardView.vue"),
    meta: { titleKey: "nav.overview" },
  },
  {
    path: "/global",
    name: "global",
    component: () => import("@/features/global/GlobalConfigView.vue"),
    meta: { titleKey: "nav.global", searchable: true },
  },
  {
    path: "/projects",
    name: "projects",
    component: () => import("@/features/projects/ProjectsView.vue"),
    meta: { titleKey: "nav.projects", searchable: true },
  },
  {
    path: "/projects/:id",
    name: "project-detail",
    component: () => import("@/features/projects/ProjectDetailView.vue"),
    meta: { titleKey: "nav.project", searchable: true },
  },
  {
    path: "/skills",
    name: "skills",
    component: () => import("@/features/skills/SkillsView.vue"),
    meta: { titleKey: "nav.skills", searchable: true },
  },
  {
    path: "/rules",
    name: "rules",
    component: () => import("@/features/rules/RulesView.vue"),
    meta: { titleKey: "nav.rules", searchable: true },
  },
  {
    path: "/agents",
    name: "agents",
    component: () => import("@/features/agents/AgentsView.vue"),
    meta: { titleKey: "nav.agents", searchable: true },
  },
  {
    path: "/hooks",
    name: "hooks",
    component: () => import("@/features/hooks/HooksView.vue"),
    meta: { titleKey: "nav.hooks", searchable: true },
  },
  {
    path: "/mcp",
    name: "mcp",
    component: () => import("@/features/mcp/McpView.vue"),
    meta: { titleKey: "nav.mcp", searchable: true },
  },
  {
    path: "/plugins",
    name: "plugins",
    component: () => import("@/features/plugins/PluginsView.vue"),
    meta: { titleKey: "nav.plugins", searchable: true },
  },
  {
    path: "/permissions",
    name: "permissions",
    component: () => import("@/features/permissions/PermissionsView.vue"),
    meta: { titleKey: "nav.permissions", searchable: true },
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("@/features/settings/SettingsView.vue"),
    meta: { titleKey: "nav.settings", searchable: true },
  },
  {
    path: "/memory",
    name: "memory",
    component: () => import("@/features/memory/MemoryView.vue"),
    meta: { titleKey: "nav.memory", searchable: true },
  },
  {
    path: "/app-settings",
    name: "app-settings",
    component: () => import("@/features/app-settings/AppSettingsView.vue"),
    meta: { titleKey: "nav.appSettings" },
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
