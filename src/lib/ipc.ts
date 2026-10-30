import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  ArtifactContent,
  BackendScanSnapshot,
  ConfigArtifact,
  Conflict,
  DashboardStats,
  Preferences,
  ProjectInfo,
  RecentChange,
  ResolvedSetting,
  SearchFilters,
  SettingsDiff,
} from "@/types/config";

/** True when running inside the Tauri webview (false in plain `vite dev`). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export class IpcError extends Error {
  constructor(
    public command: string,
    message: string,
  ) {
    super(message);
    this.name = "IpcError";
  }
}

function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object") {
    const m = (e as { message?: unknown }).message;
    if (typeof m === "string") return m;
    try {
      return JSON.stringify(e);
    } catch {
      /* ignore */
    }
  }
  return String(e);
}

export async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri()) {
    throw new IpcError(command, "Tauri runtime not available");
  }
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw new IpcError(command, errorMessage(e));
  }
}

export const ipc = {
  getAppInfo: () => call<AppInfo>("get_app_info"),
  getDashboard: () => call<DashboardStats>("get_dashboard"),
  /** Returns the raw nested snapshot; flatten with `normalizeSnapshot`. */
  runScan: () => call<BackendScanSnapshot>("run_scan"),

  listScanRoots: () => call<string[]>("list_scan_roots"),
  addScanRoot: (path: string) => call<string[]>("add_scan_root", { path }),
  removeScanRoot: (path: string) => call<string[]>("remove_scan_root", { path }),

  listProjects: () => call<ProjectInfo[]>("list_projects"),
  addProject: (path: string) => call<ProjectInfo[]>("add_project", { path }),
  /** Removes from the app list only — never deletes files. */
  removeProject: (path: string) => call<ProjectInfo[]>("remove_project", { path }),
  pickDirectory: () => call<string | null>("pick_directory"),

  /** `path` is the filesystem path of the artifact (not its id). */
  getArtifactContent: (path: string) =>
    call<ArtifactContent>("get_artifact_content", { path }),
  getEffectiveSettings: (projectPath?: string | null) =>
    call<ResolvedSetting[]>("get_effective_settings", {
      projectPath: projectPath ?? null,
    }),
  getConflicts: () => call<Conflict[]>("get_conflicts"),
  getSettingsDiff: (projectPath: string) =>
    call<SettingsDiff>("get_settings_diff", { projectPath }),
  getRecentChanges: (limit = 50) =>
    call<RecentChange[]>("get_recent_changes", { limit }),

  startWatching: () => call<string[]>("start_watching"),
  stopWatching: () => call<boolean>("stop_watching"),

  searchArtifacts: (query: string, filters: SearchFilters = {}) =>
    call<ConfigArtifact[]>("search_artifacts", {
      query,
      kind: filters.kind ?? null,
      scope: filters.scope ?? null,
    }),

  getPreferences: () => call<Preferences>("get_preferences"),
  setPreferences: (prefs: Preferences) =>
    call<Preferences>("set_preferences", { prefs }),
};

export const CONFIG_CHANGED_EVENT = "config-changed";

/** Subscribe to backend `config-changed` events. No-op outside Tauri. */
export async function onConfigChanged(cb: () => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen(CONFIG_CHANGED_EVENT, () => cb());
  } catch {
    return () => {};
  }
}
