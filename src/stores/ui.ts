import { defineStore } from "pinia";
import { ref } from "vue";

export const useUiStore = defineStore("ui", () => {
  /** Header search query — filters the current list view. */
  const search = ref("");
  /** Project chosen in Settings / Permissions views. */
  const selectedProjectPath = ref<string | null>(null);
  /** Changes received via `config-changed` since the user last looked. */
  const unseenChanges = ref(0);
  /** Artifact id to preselect when a view mounts (set by global search). */
  const focusArtifactId = ref<string | null>(null);

  function clearSearch() {
    search.value = "";
  }

  function markChangesSeen() {
    unseenChanges.value = 0;
  }

  return {
    search,
    selectedProjectPath,
    unseenChanges,
    focusArtifactId,
    clearSearch,
    markChangesSeen,
  };
});
