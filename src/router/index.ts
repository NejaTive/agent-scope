import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

declare module "vue-router" {
  interface RouteMeta {
    title?: string;
    /** Whether the header search box filters this view. */
    searchable?: boolean;
  }
}

export const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "overview",
    component: () => import("@/features/dashboard/DashboardView.vue"),
    meta: { title: "Overview" },
  },
  {
    path: "/global",
    name: "global",
    component: () => import("@/features/global/GlobalConfigView.vue"),
    meta: { title: "Global Configuration", searchable: true },
  },
  {
    path: "/projects",
    name: "projects",
    component: () => import("@/features/projects/ProjectsView.vue"),
    meta: { title: "Projects", searchable: true },
  },
  {
    path: "/projects/:id",
    name: "project-detail",
    component: () => import("@/features/projects/ProjectDetailView.vue"),
    meta: { title: "Project", searchable: true },
  },
  {
    path: "/skills",
    name: "skills",
    component: () => import("@/features/skills/SkillsView.vue"),
    meta: { title: "Skills", searchable: true },
  },
  {
    path: "/rules",
    name: "rules",
    component: () => import("@/features/rules/RulesView.vue"),
    meta: { title: "Rules", searchable: true },
  },
  {
    path: "/agents",
    name: "agents",
    component: () => import("@/features/agents/AgentsView.vue"),
    meta: { title: "Agents", searchable: true },
  },
  {
    path: "/hooks",
    name: "hooks",
    component: () => import("@/features/hooks/HooksView.vue"),
    meta: { title: "Hooks", searchable: true },
  },
  {
    path: "/mcp",
    name: "mcp",
    component: () => import("@/features/mcp/McpView.vue"),
    meta: { title: "MCP Servers", searchable: true },
  },
  {
    path: "/plugins",
    name: "plugins",
    component: () => import("@/features/plugins/PluginsView.vue"),
    meta: { title: "Plugins", searchable: true },
  },
  {
    path: "/permissions",
    name: "permissions",
    component: () => import("@/features/permissions/PermissionsView.vue"),
    meta: { title: "Permissions", searchable: true },
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("@/features/settings/SettingsView.vue"),
    meta: { title: "Settings", searchable: true },
  },
  {
    path: "/effective",
    name: "effective",
    component: () => import("@/features/effective/EffectiveView.vue"),
    meta: { title: "Effective Config", searchable: true },
  },
  {
    path: "/memory",
    name: "memory",
    component: () => import("@/features/memory/MemoryView.vue"),
    meta: { title: "Memory", searchable: true },
  },
  {
    path: "/app-settings",
    name: "app-settings",
    component: () => import("@/features/app-settings/AppSettingsView.vue"),
    meta: { title: "App Settings" },
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
