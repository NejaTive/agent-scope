import { defineStore } from "pinia";
import { ref, watch } from "vue";
import { applyLocale, detectBrowserLocale, isAppLocale } from "@/i18n";
import { ipc } from "@/lib/ipc";
import { errMsg } from "@/lib/normalize";
import type { AppLocale, Preferences, ThemePreference } from "@/types/config";
import { hasTrackedProduct } from "@/lib/tooling";
import { useScanStore } from "./scan";

const LS_KEY = "agent-scope:preferences";

const DEFAULTS: Preferences = {
  theme: "dark",
  locale: "ru",
  watchOnStart: false,
  notifications: false,
  allowEditing: false,
  trackClaude: true,
  trackCursor: true,
  trackCodex: true,
};

const THEMES: ThemePreference[] = ["dark", "light", "system"];

function sanitize(p: Partial<Preferences>): Partial<Preferences> {
  const out: Partial<Preferences> = {};
  if (p.theme && THEMES.includes(p.theme)) out.theme = p.theme;
  if (isAppLocale(p.locale)) out.locale = p.locale;
  if (typeof p.watchOnStart === "boolean") out.watchOnStart = p.watchOnStart;
  if (typeof p.notifications === "boolean") out.notifications = p.notifications;
  if (typeof p.allowEditing === "boolean") out.allowEditing = p.allowEditing;
  if (typeof p.trackClaude === "boolean") out.trackClaude = p.trackClaude;
  if (typeof p.trackCursor === "boolean") out.trackCursor = p.trackCursor;
  if (typeof p.trackCodex === "boolean") out.trackCodex = p.trackCodex;
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

/**
 * Initial prefs: defaults → browser locale (only when no saved locale) → localStorage.
 * `detectBrowserLocale` runs only when neither localStorage nor defaults override;
 * product default remains `ru` for existing users who already have prefs without `locale`
 * once backend/local merge supplies nothing — we still default to `ru` via DEFAULTS,
 * and only use navigator when localStorage is empty (first launch).
 */
function initialPrefs(): Preferences {
  const local = readLocal();
  const base: Preferences = { ...DEFAULTS };
  if (!("locale" in local) && !localStorage.getItem(LS_KEY)) {
    base.locale = detectBrowserLocale();
  }
  return { ...base, ...local };
}

export const usePreferencesStore = defineStore("preferences", () => {
  const prefs = ref<Preferences>(initialPrefs());
  const loaded = ref(false);
  const error = ref<string | null>(null);
  /** i18n key for last tooling validation error (translated in UI). */
  const toolingErrorKey = ref<string | null>(null);

  applyLocale(prefs.value.locale);

  watch(
    () => prefs.value.locale,
    (locale) => applyLocale(locale),
    { immediate: true },
  );

  async function load() {
    try {
      const remote = await ipc.getPreferences();
      if (remote && typeof remote === "object") {
        const sanitized = sanitize(remote);
        // Older DBs / backends without locale: keep current (default ru or browser).
        prefs.value = { ...prefs.value, ...sanitized };
        writeLocal(prefs.value);
      }
      error.value = null;
      toolingErrorKey.value = null;
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
    if (!hasTrackedProduct(next)) {
      toolingErrorKey.value = "settings.toolingLastError";
      error.value = null;
      return;
    }

    const prev = { ...prefs.value };
    const watchChanged =
      patch.watchOnStart !== undefined && patch.watchOnStart !== prefs.value.watchOnStart;
    const toolingChanged =
      (patch.trackClaude !== undefined && patch.trackClaude !== prefs.value.trackClaude) ||
      (patch.trackCursor !== undefined && patch.trackCursor !== prefs.value.trackCursor) ||
      (patch.trackCodex !== undefined && patch.trackCodex !== prefs.value.trackCodex);

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
      toolingErrorKey.value = null;
      savedOk = true;
    } catch (e) {
      error.value = errMsg(e);
      toolingErrorKey.value = null;
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

  function setLocale(locale: AppLocale) {
    return update({ locale });
  }

  return {
    prefs,
    loaded,
    error,
    toolingErrorKey,
    load,
    update,
    setTheme,
    setLocale,
    applyWatching,
  };
});
