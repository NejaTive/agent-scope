import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "@/lib/ipc";
import { errMsg } from "@/lib/normalize";
import type { Preferences, ThemePreference } from "@/types/config";
import { useScanStore } from "./scan";

const LS_KEY = "agent-scope:preferences";

const DEFAULTS: Preferences = {
  theme: "dark",
  watchOnStart: false,
  notifications: false,
  allowEditing: false,
  trackClaude: true,
  trackCursor: true,
};

const THEMES: ThemePreference[] = ["dark", "light", "system"];

const LAST_TOOLING_ERROR =
  "Нужно оставить включённым хотя бы один AI-инструмент";

function sanitize(p: Partial<Preferences>): Partial<Preferences> {
  const out: Partial<Preferences> = {};
  if (p.theme && THEMES.includes(p.theme)) out.theme = p.theme;
  if (typeof p.watchOnStart === "boolean") out.watchOnStart = p.watchOnStart;
  if (typeof p.notifications === "boolean") out.notifications = p.notifications;
  if (typeof p.allowEditing === "boolean") out.allowEditing = p.allowEditing;
  if (typeof p.trackClaude === "boolean") out.trackClaude = p.trackClaude;
  if (typeof p.trackCursor === "boolean") out.trackCursor = p.trackCursor;
  return out;
}

function readLocal(): Partial<Preferences> {
  try {
    const raw = localStorage.getItem(LS_KEY);
    return raw ? sanitize(JSON.parse(raw) as Partial<Preferences>) : {};
  } catch {
    return {};
  }
}

function writeLocal(p: Preferences) {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify(p));
  } catch {
    /* ignore */
  }
}

export const usePreferencesStore = defineStore("preferences", () => {
  const prefs = ref<Preferences>({ ...DEFAULTS, ...readLocal() });
  const loaded = ref(false);
  const error = ref<string | null>(null);

  async function load() {
    try {
      const remote = await ipc.getPreferences();
      if (remote && typeof remote === "object") {
        prefs.value = { ...prefs.value, ...sanitize(remote) };
        writeLocal(prefs.value);
      }
      error.value = null;
    } catch (e) {
      // Backend not ready: keep local/default values.
      error.value = errMsg(e);
    } finally {
      loaded.value = true;
    }
  }

  /** Start / stop the backend file watcher. Failures are surfaced via `error`. */
  async function applyWatching(on: boolean) {
    try {
      if (on) await ipc.startWatching();
      else await ipc.stopWatching();
    } catch (e) {
      error.value = errMsg(e);
    }
  }

  async function update(patch: Partial<Preferences>) {
    const next: Preferences = { ...prefs.value, ...patch };
    if (!next.trackClaude && !next.trackCursor) {
      error.value = LAST_TOOLING_ERROR;
      return;
    }

    const prev = { ...prefs.value };
    const watchChanged =
      patch.watchOnStart !== undefined && patch.watchOnStart !== prefs.value.watchOnStart;
    const toolingChanged =
      (patch.trackClaude !== undefined && patch.trackClaude !== prefs.value.trackClaude) ||
      (patch.trackCursor !== undefined && patch.trackCursor !== prefs.value.trackCursor);

    prefs.value = next;
    writeLocal(prefs.value);
    let savedOk = false;
    try {
      const saved = await ipc.setPreferences({ ...prefs.value });
      if (saved && typeof saved === "object") {
        prefs.value = { ...prefs.value, ...sanitize(saved) };
        writeLocal(prefs.value);
      }
      error.value = null;
      savedOk = true;
    } catch (e) {
      error.value = errMsg(e);
      // Backend rejected — revert optimistic patch.
      prefs.value = prev;
      writeLocal(prefs.value);
    }
    if (!savedOk) return;
    if (watchChanged) await applyWatching(prefs.value.watchOnStart);
    if (toolingChanged) {
      const scan = useScanStore();
      await scan.refresh({ silent: true });
      if (prefs.value.watchOnStart) {
        await applyWatching(false);
        await applyWatching(true);
      }
    }
  }

  function setTheme(theme: ThemePreference) {
    return update({ theme });
  }

  return { prefs, loaded, error, load, update, setTheme, applyWatching };
});
