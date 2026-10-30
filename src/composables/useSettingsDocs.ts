import { computed, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { ipc } from "@/lib/ipc";
import { errMsg, isProjectScope, normalizeContent } from "@/lib/normalize";
import { useScanStore } from "@/stores/scan";
import { useUiStore } from "@/stores/ui";
import type { ConfigArtifact } from "@/types/config";

export interface SettingsDoc {
  artifact: ConfigArtifact;
  data: Record<string, unknown> | null;
  error: string | null;
}

/**
 * Loads + parses scanned settings JSON files. Optionally limited to the
 * project selected in the UI store (user/managed/plugin scopes always shown).
 */
export function useSettingsDocs() {
  const scan = useScanStore();
  const ui = useUiStore();
  const { selectedProjectPath } = storeToRefs(ui);

  const docs = ref<SettingsDoc[]>([]);
  const loading = ref(false);
  let token = 0;

  const artifacts = computed(() =>
    scan.settingsArtifacts.filter((a) => {
      if (!isProjectScope(a.scope)) return true;
      return !selectedProjectPath.value || a.projectPath === selectedProjectPath.value;
    }),
  );

  async function load() {
    const my = ++token;
    loading.value = true;
    const out = await Promise.all(
      artifacts.value.map(async (artifact): Promise<SettingsDoc> => {
        if (artifact.status === "missing") return { artifact, data: null, error: null };
        try {
          const raw = await ipc.getArtifactContent(artifact.path);
          const { text: content } = normalizeContent(artifact.path, raw);
          if (!content.trim()) return { artifact, data: {}, error: null };
          const parsed: unknown = JSON.parse(content);
          if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
            return { artifact, data: parsed as Record<string, unknown>, error: null };
          }
          return { artifact, data: null, error: "Settings file is not a JSON object" };
        } catch (e) {
          return { artifact, data: null, error: errMsg(e) };
        }
      }),
    );
    if (my === token) {
      docs.value = out;
      loading.value = false;
    }
  }

  watch(
    [() => scan.snapshot, selectedProjectPath],
    () => {
      void load();
    },
    { immediate: true },
  );

  return { docs, loading, reload: load };
}
