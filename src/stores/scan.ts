import { defineStore } from "pinia";
import { computed, ref, shallowRef } from "vue";
import { ipc, onConfigChanged } from "@/lib/ipc";
import {
  emptySnapshot,
  errMsg,
  isProjectScope,
  normalizeProjectInfo,
  normalizeScanRoots,
  normalizeSnapshot,
} from "@/lib/normalize";
import type {
  AppInfo,
  ArtifactKind,
  ConfigArtifact,
  Conflict,
  DashboardStats,
  DashboardSummary,
  ProjectInfo,
  RecentChange,
  ScanRoot,
  ScanSnapshot,
} from "@/types/config";
import { useUiStore } from "./ui";

export const useScanStore = defineStore("scan", () => {
  const ui = useUiStore();

  const snapshot = shallowRef<ScanSnapshot>(emptySnapshot());
  const dashboardRaw = ref<DashboardStats | null>(null);
  const recentChanges = ref<RecentChange[]>([]);
  const conflicts = ref<Conflict[]>([]);
  const projectsList = ref<ProjectInfo[]>([]);
  const scanRoots = ref<ScanRoot[]>([]);
  const appInfo = ref<AppInfo | null>(null);

  const scanning = ref(false);
  const initialized = ref(false);
  const backendAvailable = ref(true);
  const error = ref<string | null>(null);

  // ---- derived ------------------------------------------------------------
  const artifacts = computed(() => snapshot.value.artifacts);
  const skillEntries = computed(() => snapshot.value.skills);
  const skills = computed(() => skillEntries.value.filter((s) => s.kind === "skill"));
  const commands = computed(() => skillEntries.value.filter((s) => s.kind === "command"));
  const rules = computed(() => snapshot.value.rules);
  const agents = computed(() => snapshot.value.agents);
  const hooks = computed(() => snapshot.value.hooks);
  const mcpServers = computed(() => snapshot.value.mcpServers);
  const scanErrors = computed(() => snapshot.value.errors);

  const projects = computed<ProjectInfo[]>(() =>
    projectsList.value.length ? projectsList.value : snapshot.value.projects,
  );
  const globalMeta = computed(() => snapshot.value.global);

  function artifactsOfKind(...kinds: ArtifactKind[]): ConfigArtifact[] {
    return artifacts.value.filter((a) => kinds.includes(a.kind));
  }

  const plugins = computed(() => artifactsOfKind("plugin"));
  const settingsArtifacts = computed(() => artifactsOfKind("settings"));

  /**
   * Dashboard numbers: backend `DashboardStats` (from `get_dashboard`, falling back to the
   * snapshot's own stats) plus a global/project skill split computed from snapshot skills.
   */
  const dashboard = computed<DashboardSummary>(() => {
    const s = snapshot.value;
    const stats: DashboardStats = dashboardRaw.value ?? s.stats;
    const realSkills = s.skills.filter((x) => x.kind === "skill");
    const projectSkills = realSkills.filter((x) => isProjectScope(x.scope)).length;
    return {
      ...stats,
      recentChanges: stats.recentChanges?.length ? stats.recentChanges : recentChanges.value,
      projectCount: stats.projectCount || projects.value.length,
      conflictCount: stats.conflictCount || conflicts.value.length,
      globalSkills: realSkills.length - projectSkills,
      projectSkills,
    };
  });

  const lastScanAt = computed(() => dashboard.value.lastScanAt ?? snapshot.value.scannedAt);

  // ---- actions ------------------------------------------------------------
  async function loadMeta() {
    const results = await Promise.allSettled([
      ipc.getAppInfo(),
      ipc.listScanRoots(),
      ipc.listProjects(),
      ipc.getDashboard(),
      ipc.getRecentChanges(50),
      ipc.getConflicts(),
    ]);
    const [info, roots, projs, dash, changes, conf] = results;
    if (info.status === "fulfilled") appInfo.value = info.value;
    if (roots.status === "fulfilled") scanRoots.value = normalizeScanRoots(roots.value);
    if (projs.status === "fulfilled" && Array.isArray(projs.value)) {
      projectsList.value = projs.value.map(normalizeProjectInfo);
    }
    if (dash.status === "fulfilled") dashboardRaw.value = dash.value;
    if (changes.status === "fulfilled" && Array.isArray(changes.value))
      recentChanges.value = changes.value;
    if (conf.status === "fulfilled" && Array.isArray(conf.value)) conflicts.value = conf.value;

    const allFailed = results.every((r) => r.status === "rejected");
    backendAvailable.value = !allFailed;
    if (allFailed) {
      const first = results[0] as PromiseRejectedResult;
      error.value = errMsg(first.reason);
    }
  }

  /** Run a scan and refresh derived data. `silent` avoids the scanning indicator. */
  let running = false;
  let pending = false;
  async function refresh(opts: { silent?: boolean } = {}) {
    if (running) {
      pending = true;
      return;
    }
    running = true;
    if (!opts.silent) scanning.value = true;
    try {
      const raw = await ipc.runScan();
      snapshot.value = normalizeSnapshot(raw);
      conflicts.value = snapshot.value.conflicts;
      error.value = null;
      backendAvailable.value = true;
      await loadMeta();
    } catch (e) {
      error.value = errMsg(e);
      await loadMeta().catch(() => {});
    } finally {
      scanning.value = false;
      initialized.value = true;
      running = false;
      if (pending) {
        pending = false;
        void refresh({ silent: true });
      }
    }
  }

  async function init(scanOnStart = true) {
    if (scanOnStart) {
      await refresh();
    } else {
      await loadMeta();
      initialized.value = true;
    }
  }

  // Debounced soft refresh on backend `config-changed` events.
  let timer: ReturnType<typeof setTimeout> | null = null;
  let unlisten: (() => void) | null = null;

  async function listenForChanges() {
    if (unlisten) return;
    unlisten = await onConfigChanged(() => {
      ui.unseenChanges += 1;
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        void refresh({ silent: true });
      }, 400);
    });
  }

  function stopListening() {
    if (timer) clearTimeout(timer);
    unlisten?.();
    unlisten = null;
  }

  // ---- project management (app list only; never touches files) -------------
  async function addProjectFromPicker(): Promise<string | null> {
    try {
      const dir = await ipc.pickDirectory();
      if (!dir) return null;
      await ipc.addProject(dir);
      await refresh({ silent: true });
      return dir;
    } catch (e) {
      error.value = errMsg(e);
      return null;
    }
  }

  async function addScanRootFromPicker(): Promise<string | null> {
    try {
      const dir = await ipc.pickDirectory();
      if (!dir) return null;
      await ipc.addScanRoot(dir);
      await refresh({ silent: true });
      return dir;
    } catch (e) {
      error.value = errMsg(e);
      return null;
    }
  }

  async function removeScanRoot(path: string) {
    try {
      await ipc.removeScanRoot(path);
      await refresh({ silent: true });
    } catch (e) {
      error.value = errMsg(e);
    }
  }

  async function removeProject(path: string) {
    try {
      await ipc.removeProject(path);
      if (ui.selectedProjectPath === path) ui.selectedProjectPath = null;
      await refresh({ silent: true });
    } catch (e) {
      error.value = errMsg(e);
    }
  }

  return {
    snapshot,
    dashboard,
    recentChanges,
    conflicts,
    scanRoots,
    appInfo,
    scanning,
    initialized,
    backendAvailable,
    error,
    artifacts,
    skillEntries,
    skills,
    commands,
    rules,
    agents,
    hooks,
    mcpServers,
    plugins,
    settingsArtifacts,
    scanErrors,
    projects,
    lastScanAt,
    globalMeta,
    artifactsOfKind,
    init,
    refresh,
    loadMeta,
    listenForChanges,
    stopListening,
    addProjectFromPicker,
    addScanRootFromPicker,
    removeScanRoot,
    removeProject,
  };
});
