import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { ipc } from "@/lib/ipc";
import { errMsg } from "@/lib/normalize";
import type { UpdateCheckResult } from "@/types/config";

export type UpdateCheckStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "update-available"
  | "error";

export const useUpdatesStore = defineStore("updates", () => {
  const status = ref<UpdateCheckStatus>("idle");
  const result = ref<UpdateCheckResult | null>(null);
  const error = ref<string | null>(null);
  /** Ensures the quiet start check runs at most once per session. */
  let quietStarted = false;

  const updateAvailable = computed(
    () => status.value === "update-available" && !!result.value?.updateAvailable,
  );
  /** Tag without leading `v`/`V` for UI copy («Обновление 0.4.0»). */
  const latestVersion = computed(() => {
    const raw = result.value?.latestVersion;
    if (!raw) return null;
    return raw.replace(/^[vV]/, "");
  });
  const releaseUrl = computed(() => result.value?.releaseUrl ?? null);

  function applySuccess(res: UpdateCheckResult) {
    result.value = res;
    error.value = null;
    if (res.error && !res.updateAvailable) {
      status.value = "error";
      error.value = res.error;
      return;
    }
    status.value = res.updateAvailable ? "update-available" : "up-to-date";
  }

  /**
   * Manual check from settings — surfaces checking / error / result states.
   */
  async function check() {
    status.value = "checking";
    error.value = null;
    try {
      applySuccess(await ipc.checkForUpdates());
    } catch (e) {
      result.value = null;
      status.value = "error";
      error.value = errMsg(e);
    }
  }

  /**
   * Background check on app start. Failures stay silent (idle, no banner).
   * Successful `updateAvailable` still drives the sidebar banner.
   */
  async function checkQuiet() {
    if (quietStarted) return;
    quietStarted = true;
    try {
      const res = await ipc.checkForUpdates();
      if (res.error && !res.updateAvailable) {
        // Soft backend error — do not surface in UI.
        return;
      }
      result.value = res;
      error.value = null;
      status.value = res.updateAvailable ? "update-available" : "up-to-date";
    } catch {
      // Offline / missing command / network — leave idle, no banner.
    }
  }

  return {
    status,
    result,
    error,
    updateAvailable,
    latestVersion,
    releaseUrl,
    check,
    checkQuiet,
  };
});
