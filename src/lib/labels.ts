import type {
  ArtifactKind,
  Confidence,
  DiscoveryStatus,
  Scope,
} from "@/types/config";

export const SCOPE_LABEL: Record<Scope, string> = {
  managed: "Managed",
  user: "User",
  project: "Project",
  projectLocal: "Local",
  plugin: "Plugin",
  nested: "Nested",
  globalConfig: "Global config",
};

export const SCOPE_ORDER: Scope[] = [
  "managed",
  "user",
  "project",
  "projectLocal",
  "nested",
  "plugin",
  "globalConfig",
];

/** Precedence (higher wins) used for display ordering only. */
export const SCOPE_PRECEDENCE: Record<Scope, number> = {
  managed: 6,
  projectLocal: 5,
  project: 4,
  nested: 3,
  user: 2,
  plugin: 1,
  globalConfig: 0,
};

export const KIND_LABEL: Record<ArtifactKind, string> = {
  skill: "Skill",
  command: "Command",
  rule: "Rule",
  agent: "Agent",
  hook: "Hook",
  mcpServer: "MCP server",
  plugin: "Plugin",
  settings: "Settings",
  memory: "Memory",
  claudeMd: "CLAUDE.md",
  workflow: "Workflow",
  outputStyle: "Output style",
  other: "Other",
};

export const KIND_PLURAL: Record<ArtifactKind, string> = {
  skill: "Skills",
  command: "Commands",
  rule: "Rules",
  agent: "Agents",
  hook: "Hooks",
  mcpServer: "MCP servers",
  plugin: "Plugins",
  settings: "Settings",
  memory: "Memory",
  claudeMd: "CLAUDE.md",
  workflow: "Workflows",
  outputStyle: "Output styles",
  other: "Other",
};

export const STATUS_LABEL: Record<DiscoveryStatus, string> = {
  present: "Present",
  missing: "Missing",
  unreadable: "Unreadable",
  invalidJson: "Invalid JSON",
  invalidFrontmatter: "Invalid frontmatter",
  partial: "Partial",
};

export const STATUS_TONE: Record<
  DiscoveryStatus,
  "ok" | "warn" | "danger" | "neutral"
> = {
  present: "ok",
  missing: "neutral",
  unreadable: "danger",
  invalidJson: "danger",
  invalidFrontmatter: "warn",
  partial: "warn",
};

export const CONFIDENCE_LABEL: Record<Confidence, string> = {
  computed: "Computed",
  inferred: "Inferred",
  unknown: "Unknown",
};

/** Route for each artifact kind (used by global search). */
export function routeForKind(kind: ArtifactKind): string {
  switch (kind) {
    case "skill":
    case "command":
      return "/skills";
    case "rule":
      return "/rules";
    case "agent":
      return "/agents";
    case "hook":
      return "/hooks";
    case "mcpServer":
      return "/mcp";
    case "plugin":
      return "/plugins";
    case "settings":
      return "/settings";
    case "memory":
    case "claudeMd":
    case "workflow":
    case "outputStyle":
      return "/memory";
    default:
      return "/global";
  }
}
