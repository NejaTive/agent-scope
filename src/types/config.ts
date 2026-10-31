/**
 * Frontend mirror of the Rust backend IPC contract (serde `camelCase`).
 * The backend is the source of truth: keep these in sync with `src-tauri/src/models`.
 *
 * The second half of this file holds UI-only "flattened" shapes produced by
 * `normalizeSnapshot` (see `@/lib/normalize`). They are never sent to the backend.
 *
 * Timestamps arrive as ISO-8601 strings; use `formatTime` / `relativeTime`
 * from `@/lib/format` to display them.
 */

export type Timestamp = string | number | null;

// ---------------------------------------------------------------- enums

export type Scope =
  | "managed"
  | "user"
  | "project"
  | "projectLocal"
  | "plugin"
  | "nested"
  | "globalConfig";

export type ArtifactKind =
  | "settings"
  | "claudeMd"
  | "rule"
  | "skill"
  | "command"
  | "agent"
  | "hook"
  | "mcpServer"
  | "plugin"
  | "memory"
  | "outputStyle"
  | "workflow"
  | "other";

export type DiscoveryStatus =
  | "present"
  | "missing"
  | "unreadable"
  | "invalidJson"
  | "invalidFrontmatter"
  | "partial";

export type Confidence = "computed" | "inferred" | "unknown";

/** Claude Code vs Cursor product tagging (`product` field). */
export type Tooling = "claude" | "cursor";
/** @deprecated Prefer `Tooling`. */
export type Product = Tooling;

export type ConflictKind = "duplicateName" | "mcpMismatch" | "settingOverride";
export type ConflictSeverity = "info" | "warning";

export type ThemePreference = "dark" | "light" | "system";

/** Arbitrary JSON value (frontmatter, config, metadata, ...). */
export type Json = unknown;

// ---------------------------------------------------------------- backend structs

export interface ConfigArtifact {
  id: string;
  kind: ArtifactKind;
  scope: Scope;
  name: string;
  /** Filesystem path; also the key for `get_artifact_content`. */
  path: string;
  projectPath?: string | null;
  status: DiscoveryStatus;
  sizeBytes?: number | null;
  /** ISO timestamp. */
  modifiedAt?: string | null;
  description?: string | null;
  error?: string | null;
  /** Relative file listing for directory-like artifacts (skills). */
  children: string[];
  metadata: Json;
  /** Claude Code vs Cursor; omit / missing → UI treats as `"claude"`. */
  product?: Tooling | null;
}

export interface ProjectInfo {
  path: string;
  name: string;
  hasClaudeDir: boolean;
  hasClaudeMd: boolean;
  hasMcpJson: boolean;
  /** Present when the project has a `.cursor` directory. */
  hasCursorDir: boolean;
  /** Present when the project has `.cursor/mcp.json`. */
  hasCursorMcpJson: boolean;
  pinned: boolean;
  discovered: boolean;
  artifactCount: number;
}

export interface ScanError {
  path: string;
  scope?: Scope | null;
  status: DiscoveryStatus;
  message: string;
}

export interface SettingsLayer {
  scope: Scope;
  path: string;
  projectPath?: string | null;
  value: Json;
}

export interface HookEntry {
  event: string;
  matcher?: string | null;
  /** `command`, `prompt`, `agent`, ... */
  hookType: string;
  command?: string | null;
  prompt?: string | null;
  timeout?: number | null;
  scope: Scope;
  sourcePath: string;
  projectPath?: string | null;
  product?: Product | null;
  details: Json;
}

export interface McpServerEntry {
  name: string;
  scope: Scope;
  sourcePath: string;
  projectPath?: string | null;
  product?: Product | null;
  /** `.mcp.json`, `~/.claude.json`, `~/.claude.json#projects`, `managed-mcp.json` */
  origin: string;
  /** `stdio`, `http`, `sse`, ... */
  serverType: string;
  command?: string | null;
  args: string[];
  url?: string | null;
  /** Masked server config. */
  config: Json;
}

export interface SkillEntry {
  name: string;
  /** `skill` or `command` */
  kind: ArtifactKind;
  description?: string | null;
  scope: Scope;
  path: string;
  projectPath?: string | null;
  product?: Product | null;
  status: DiscoveryStatus;
  frontmatter: Json;
  files: string[];
}

export interface RuleEntry {
  name: string;
  scope: Scope;
  path: string;
  projectPath?: string | null;
  product?: Product | null;
  paths: string[];
  conditional: boolean;
  status: DiscoveryStatus;
}

export interface AgentEntry {
  name: string;
  description?: string | null;
  scope: Scope;
  path: string;
  projectPath?: string | null;
  product?: Product | null;
  tools: string[];
  model?: string | null;
  /** name + description both present */
  valid: boolean;
  status: DiscoveryStatus;
  frontmatter: Json;
}

/** Everything collected for one scope container (global or one project). */
export interface Collection {
  artifacts: ConfigArtifact[];
  hooks: HookEntry[];
  mcpServers: McpServerEntry[];
  skills: SkillEntry[];
  rules: RuleEntry[];
  agents: AgentEntry[];
  settingsLayers: SettingsLayer[];
  errors: ScanError[];
}

export interface GlobalBundle extends Collection {
  configDir: string;
  managedDir: string;
  claudeJsonPath: string;
  /** User Cursor config root (`~/.cursor`), when known. */
  cursorConfigDir?: string | null;
  credentialsPresent: boolean;
}

export interface ProjectBundle extends Collection {
  project: ProjectInfo;
}

export interface ConflictEntry {
  scope: Scope;
  path: string;
  kind?: ArtifactKind | null;
  value?: Json;
}

export interface Conflict {
  id: string;
  kind: ConflictKind;
  severity: ConflictSeverity;
  key: string;
  message: string;
  projectPath?: string | null;
  entries: ConflictEntry[];
}

export interface SettingSource {
  scope: Scope;
  path: string;
  value: Json;
}

export interface ResolvedSetting {
  key: string;
  value: Json;
  /** Winning scope (highest precedence contributor). */
  scope: Scope;
  sourcePath?: string | null;
  confidence: Confidence;
  explanation: string;
  /** True when the value is a list-merge of multiple scopes. */
  merged: boolean;
  /** All scopes that define this key, highest precedence first. */
  sources: SettingSource[];
}

export interface SettingsDiffRow {
  key: string;
  user?: Json;
  project?: Json;
  local?: Json;
  differs: boolean;
}

export interface SettingsDiff {
  projectPath: string;
  userPath?: string | null;
  projectSettingsPath?: string | null;
  localPath?: string | null;
  user?: Json;
  project?: Json;
  local?: Json;
  rows: SettingsDiffRow[];
}

export interface RecentChange {
  id: number;
  path: string;
  /** e.g. created | modified | deleted (free-form string from the watcher). */
  kind: string;
  /** ISO timestamp. */
  ts: string;
  hash?: string | null;
}

export interface DashboardStats {
  totalArtifacts: number;
  projectCount: number;
  skills: number;
  commands: number;
  agents: number;
  rules: number;
  hooks: number;
  mcpServers: number;
  plugins: number;
  memories: number;
  conflictCount: number;
  errorCount: number;
  byKind: Record<string, number>;
  byScope: Record<string, number>;
  lastScanAt?: string | null;
  recentChanges: RecentChange[];
}

/** Raw snapshot as returned by `run_scan`. */
export interface BackendScanSnapshot {
  id: string;
  scannedAt: string;
  durationMs: number;
  global: GlobalBundle;
  projects: ProjectBundle[];
  conflicts: Conflict[];
  errors: ScanError[];
  stats: DashboardStats;
}

export interface ArtifactContent {
  text: string;
  language: string;
}

/** Raw (unmasked) content for Edit mode. */
export interface ArtifactContentForEdit {
  text: string;
  language: string;
  contentHash: string;
  writable: boolean;
}

export interface SaveArtifactResult {
  /** Previous on-disk content for one-level UI undo (no `.bak` on disk). */
  previousContent: string;
  contentHash: string;
}

export interface RenameArtifactResult {
  newPath: string;
}

export interface DeleteArtifactResult {
  deletedPath: string;
}

export interface AppInfo {
  name: string;
  version: string;
}

export interface Preferences {
  theme: ThemePreference;
  watchOnStart: boolean;
  notifications: boolean;
  /** Opt-in: allow writing allowlisted config files. Default false. */
  allowEditing: boolean;
}

export type AppPreferences = Preferences;

export interface SearchFilters {
  kind?: ArtifactKind | null;
  scope?: Scope | null;
  product?: Product | null;
}

/** `search_artifacts` returns plain artifacts. */
export type SearchHit = ConfigArtifact;

// ---------------------------------------------------------------- UI-flattened shapes

/** Scan roots are plain path strings on the backend. */
export interface ScanRoot {
  path: string;
}

/** Fields added by `normalizeSnapshot` to every flattened entry. */
export interface UiEntryExtras {
  /** Stable list key, e.g. `skill:/path/SKILL.md`. */
  id: string;
  /** Key for `get_artifact_content` (always a filesystem path). */
  artifactId: string;
  /** Last-modified time of the backing file, looked up from artifacts. */
  mtime: Timestamp;
}

export type UiSkill = SkillEntry & UiEntryExtras;
export type UiRule = RuleEntry & UiEntryExtras;
export type UiAgent = AgentEntry & UiEntryExtras;
export type UiHook = HookEntry & UiEntryExtras & {
  /** Alias of `sourcePath` (file shown in the content viewer). */
  path: string;
  status: DiscoveryStatus;
};
export type UiMcpServer = McpServerEntry & UiEntryExtras & {
  /** Alias of `sourcePath`. */
  path: string;
  status: DiscoveryStatus;
};

export interface GlobalMeta {
  configDir: string;
  managedDir: string;
  claudeJsonPath: string;
  /** User Cursor config root (`~/.cursor`), when known. */
  cursorConfigDir?: string | null;
  credentialsPresent: boolean;
}

/** UI-friendly flat snapshot (derived from the nested backend shape). */
export interface ScanSnapshot {
  id: string | null;
  scannedAt: Timestamp;
  durationMs: number | null;
  /** Global + all project artifacts. */
  artifacts: ConfigArtifact[];
  projects: ProjectInfo[];
  skills: UiSkill[];
  rules: UiRule[];
  agents: UiAgent[];
  hooks: UiHook[];
  mcpServers: UiMcpServer[];
  settingsLayers: SettingsLayer[];
  errors: ScanError[];
  conflicts: Conflict[];
  stats: DashboardStats;
  global: GlobalMeta;
}

/** Dashboard numbers shown in the UI: backend stats plus a global/project skill split. */
export interface DashboardSummary extends DashboardStats {
  globalSkills: number;
  projectSkills: number;
}

/** Content prepared for the viewers. */
export interface ViewContent extends ArtifactContent {
  path: string;
}
