import { tr } from "@/i18n";

export type SettingsCategory =
  | "General"
  | "Permissions"
  | "Environment"
  | "Hooks"
  | "Model"
  | "MCP"
  | "Plugins"
  | "Other";

export const SETTINGS_CATEGORIES: SettingsCategory[] = [
  "General",
  "Permissions",
  "Environment",
  "Hooks",
  "Model",
  "MCP",
  "Plugins",
  "Other",
];

const CATEGORY_I18N: Record<SettingsCategory, string> = {
  General: "category.general",
  Permissions: "category.permissions",
  Environment: "category.environment",
  Hooks: "category.hooks",
  Model: "category.model",
  MCP: "category.mcp",
  Plugins: "category.plugins",
  Other: "category.other",
};

function categoryLabel(cat: SettingsCategory): string {
  return tr(CATEGORY_I18N[cat]);
}

/** Locale-aware category labels (access resolves via vue-i18n). */
export const CATEGORY_LABEL: Record<SettingsCategory, string> = (() => {
  const out = {} as Record<SettingsCategory, string>;
  for (const cat of SETTINGS_CATEGORIES) {
    Object.defineProperty(out, cat, {
      enumerable: true,
      configurable: true,
      get: () => categoryLabel(cat),
    });
  }
  return out;
})();

const BY_KEY: Record<string, SettingsCategory> = {
  // general
  theme: "General",
  language: "General",
  cleanupPeriodDays: "General",
  includeCoAuthoredBy: "General",
  includeGitInstructions: "General",
  statusLine: "General",
  outputStyle: "General",
  autoUpdates: "General",
  autoUpdatesChannel: "General",
  forceLoginMethod: "General",
  forceLoginOrgUUID: "General",
  apiKeyHelper: "General",
  awsAuthRefresh: "General",
  awsCredentialExport: "General",
  spinnerTipsEnabled: "General",
  respectGitignore: "General",
  companyAnnouncements: "General",
  // permissions
  permissions: "Permissions",
  // environment
  env: "Environment",
  // hooks
  hooks: "Hooks",
  disableAllHooks: "Hooks",
  allowedHttpHookUrls: "Hooks",
  // model
  model: "Model",
  availableModels: "Model",
  modelOverrides: "Model",
  alwaysThinkingEnabled: "Model",
  effortLevel: "Model",
  // mcp
  mcpServers: "MCP",
  enableAllProjectMcpServers: "MCP",
  enabledMcpjsonServers: "MCP",
  disabledMcpjsonServers: "MCP",
  allowedMcpServers: "MCP",
  deniedMcpServers: "MCP",
  // plugins
  enabledPlugins: "Plugins",
  extraKnownMarketplaces: "Plugins",
  strictKnownMarketplaces: "Plugins",
  pluginMarketplaces: "Plugins",
};

export function categorizeSetting(key: string): SettingsCategory {
  if (BY_KEY[key]) return BY_KEY[key];
  if (/mcp/i.test(key)) return "MCP";
  if (/plugin|marketplace/i.test(key)) return "Plugins";
  if (/hook/i.test(key)) return "Hooks";
  if (/permission/i.test(key)) return "Permissions";
  if (/^model|thinking|effort/i.test(key)) return "Model";
  return "Other";
}
