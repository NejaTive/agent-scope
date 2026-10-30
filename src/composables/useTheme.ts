import { computed, onBeforeUnmount, onMounted, watch } from "vue";
import { usePreferencesStore } from "@/stores/preferences";
import type { ThemePreference } from "@/types/config";

const ORDER: ThemePreference[] = ["dark", "light", "system"];

/** Applies the `dark` class to <html> based on preference + system setting. */
export function useTheme() {
  const store = usePreferencesStore();
  const media =
    typeof window !== "undefined" && window.matchMedia
      ? window.matchMedia("(prefers-color-scheme: dark)")
      : null;

  const preference = computed(() => store.prefs.theme);
  const resolved = computed<"dark" | "light">(() => {
    if (preference.value === "system") return media?.matches ? "dark" : "light";
    return preference.value;
  });

  function apply() {
    document.documentElement.classList.toggle("dark", resolved.value === "dark");
  }

  function onSystemChange() {
    if (preference.value === "system") apply();
  }

  function cycle() {
    const next = ORDER[(ORDER.indexOf(preference.value) + 1) % ORDER.length];
    void store.setTheme(next);
  }

  onMounted(() => {
    apply();
    media?.addEventListener("change", onSystemChange);
  });
  onBeforeUnmount(() => media?.removeEventListener("change", onSystemChange));
  watch(preference, apply);
  // Re-apply once the backend preferences have loaded.
  watch(() => store.loaded, apply);

  return { preference, resolved, cycle, set: (t: ThemePreference) => store.setTheme(t) };
}
