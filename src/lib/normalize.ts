import type {
  AgentEntry,
  ArtifactContent,
  BackendScanSnapshot,
  Collection,
  ConfigArtifact,
  DashboardStats,
  DashboardSummary,
  DiscoveryStatus,
  GlobalMeta,
  HookEntry,
  McpServerEntry,
  Product,
  ProjectInfo,
  RuleEntry,
  ScanRoot,
  ScanSnapshot,
  Scope,
  SkillEntry,
  Timestamp,
  UiAgent,
  UiHook,
  UiMcpServer,
  UiRule,
  UiSkill,
  ViewContent,
} from "@/types/config";
import { resolveProduct } from "@/lib/labels";

const arr = <T>(v: T[] | undefined | null): T[] => (Array.isArray(v) ? v : []);

function withProduct<T extends { product?: Product | null }>(
  e: T,
  fallback?: Product | null,
): T & { product: Product } {
  return { ...e, product: resolveProduct(e.product ?? fallback) };
}

/** Scopes that belong to a project (as opposed to user/managed/plugin/global). */
const PROJECT_SCOPES: ReadonlySet<Scope> = new Set<Scope>(["project", "projectLocal", "nested"]);
export const isProjectScope = (s: Scope): boolean => PROJECT_SCOPES.has(s);

export function emptyStats(): DashboardStats {
  return {
    totalArtifacts: 0,
    projectCount: 0,
    skills: 0,
    commands: 0,
    agents: 0,
    rules: 0,
    hooks: 0,
    mcpServers: 0,
    plugins: 0,
    memories: 0,
    conflictCount: 0,
    errorCount: 0,
    byKind: {},
    byScope: {},
    lastScanAt: null,
    recentChanges: [],
  };
}

export function emptyDashboard(): DashboardSummary {
  return { ...emptyStats(), globalSkills: 0, projectSkills: 0 };
}

function emptyGlobalMeta(): GlobalMeta {
  return {
    configDir: "",
    managedDir: "",
    claudeJsonPath: "",
    cursorConfigDir: null,
    credentialsPresent: false,
  };
}

/** Coerce project flags so missing backend fields (e.g. `hasCursorDir`) stay safe. */
export function normalizeProjectInfo(p: ProjectInfo): ProjectInfo {
  return {
    ...p,
    hasClaudeDir: !!p.hasClaudeDir,
    hasCursorDir: !!p.hasCursorDir,
    hasClaudeMd: !!p.hasClaudeMd,
    hasMcpJson: !!p.hasMcpJson,
    hasCursorMcpJson: !!p.hasCursorMcpJson,
    pinned: !!p.pinned,
    discovered: !!p.discovered,
    artifactCount: p.artifactCount ?? 0,
  };
}

export function emptySnapshot(): ScanSnapshot {
  return {
    id: null,
    scannedAt: null,
    durationMs: null,
    artifacts: [],
    projects: [],
    skills: [],
    rules: [],
    agents: [],
    hooks: [],
    mcpServers: [],
    settingsLayers: [],
    errors: [],
    conflicts: [],
    stats: emptyStats(),
    global: emptyGlobalMeta(),
  };
}

type Partial2<T> = { [K in keyof T]?: T[K] | null };

/**
 * Flatten the nested backend snapshot (`global` + `projects[]`) into the flat
 * shape the views consume. Entry `id`s are derived (`${kind}:${path}`), and
 * `artifactId` is always the filesystem path used by `get_artifact_content`.
 */
export function normalizeSnapshot(
  raw: Partial2<BackendScanSnapshot> | null | undefined,
): ScanSnapshot {
  const base = emptySnapshot();
  if (!raw || typeof raw !== "object") return base;

  const g: Partial2<Collection> & Partial2<GlobalMeta> = raw.global ?? {};
  const bundles: Array<Partial2<Collection>> = [g, ...arr(raw.projects)];
  const projects: ProjectInfo[] = arr(raw.projects)
    .map((p) => p.project)
    .filter((p): p is ProjectInfo => !!p)
    .map(normalizeProjectInfo);

  const artifacts: ConfigArtifact[] = [];
  const settingsLayers = [] as ScanSnapshot["settingsLayers"];
  const errors = [...arr(raw.errors)];
  const rawSkills: SkillEntry[] = [];
  const rawRules: RuleEntry[] = [];
  const rawAgents: AgentEntry[] = [];
  const rawHooks: HookEntry[] = [];
  const rawMcp: McpServerEntry[] = [];

  for (const b of bundles) {
    artifacts.push(...arr(b.artifacts).map((a) => withProduct(a)));
    settingsLayers.push(...arr(b.settingsLayers));
    errors.push(...arr(b.errors));
    rawSkills.push(...arr(b.skills));
    rawRules.push(...arr(b.rules));
    rawAgents.push(...arr(b.agents));
    rawHooks.push(...arr(b.hooks));
    rawMcp.push(...arr(b.mcpServers));
  }

  const mtimeByPath = new Map<string, Timestamp>();
  const productByPath = new Map<string, Product>();
  for (const a of artifacts) {
    if (a.modifiedAt && !mtimeByPath.has(a.path)) mtimeByPath.set(a.path, a.modifiedAt);
    if (!productByPath.has(a.path)) productByPath.set(a.path, resolveProduct(a.product));
  }
  const mt = (path: string): Timestamp => mtimeByPath.get(path) ?? null;
  /** Prefer entry.product, else matching artifact, else Claude Code. */
  const productFor = (path: string, own?: Product | null): Product =>
    resolveProduct(own ?? productByPath.get(path));

  const skills: UiSkill[] = rawSkills.map((s) => ({
    ...withProduct(s, productByPath.get(s.path)),
    id: `${s.kind}:${s.path}`,
    artifactId: s.path,
    mtime: mt(s.path),
  }));
  const rules: UiRule[] = rawRules.map((r) => ({
    ...withProduct(r, productByPath.get(r.path)),
    paths: arr(r.paths),
    id: `rule:${r.path}`,
    artifactId: r.path,
    mtime: mt(r.path),
  }));
  const agents: UiAgent[] = rawAgents.map((a) => ({
    ...withProduct(a, productByPath.get(a.path)),
    tools: arr(a.tools),
    id: `agent:${a.path}`,
    artifactId: a.path,
    mtime: mt(a.path),
  }));

  // Hooks/MCP servers have no own file: they point at the declaring file.
  const sourceStatus = (path: string): DiscoveryStatus =>
    artifacts.find((a) => a.path === path)?.status ?? "present";
  const sourceProduct = (path: string, own?: Product | null): Product => productFor(path, own);

  const hooks: UiHook[] = rawHooks.map((h, i) => ({
    ...h,
    product: sourceProduct(h.sourcePath, h.product),
    id: `hook:${h.sourcePath}:${h.event}:${h.matcher ?? ""}:${i}`,
    artifactId: h.sourcePath,
    path: h.sourcePath,
    status: sourceStatus(h.sourcePath),
    mtime: mt(h.sourcePath),
  }));
  const mcpServers: UiMcpServer[] = rawMcp.map((s) => ({
    ...s,
    product: sourceProduct(s.sourcePath, s.product),
    args: arr(s.args),
    id: `mcp:${s.sourcePath}:${s.projectPath ?? ""}:${s.name}`,
    artifactId: s.sourcePath,
    path: s.sourcePath,
    status: sourceStatus(s.sourcePath),
    mtime: mt(s.sourcePath),
  }));

  return {
    ...base,
    id: raw.id ?? null,
    scannedAt: raw.scannedAt ?? null,
    durationMs: raw.durationMs ?? null,
    artifacts,
    projects,
    skills,
    rules,
    agents,
    hooks,
    mcpServers,
    settingsLayers,
    errors,
    conflicts: arr(raw.conflicts),
    stats: { ...emptyStats(), ...(raw.stats ?? {}) },
    global: {
      configDir: g.configDir ?? "",
      managedDir: g.managedDir ?? "",
      claudeJsonPath: g.claudeJsonPath ?? "",
      cursorConfigDir: g.cursorConfigDir ?? null,
      credentialsPresent: !!g.credentialsPresent,
    },
  };
}

/** `list_scan_roots` returns plain path strings. */
export function normalizeScanRoots(raw: Array<ScanRoot | string> | null | undefined): ScanRoot[] {
  return arr(raw).map((r) => (typeof r === "string" ? { path: r } : r));
}

/** Map backend `{ text, language }` to what the viewers consume. */
export function normalizeContent(
  path: string,
  raw: Partial<ArtifactContent> | string | null | undefined,
): ViewContent {
  if (typeof raw === "string") return { path, text: raw, language: "plaintext" };
  return { path, text: raw?.text ?? "", language: raw?.language ?? "plaintext" };
}

export function errMsg(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  return String(e);
}
