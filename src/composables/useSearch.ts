import { computed, ref, type Ref } from "vue";
import { storeToRefs } from "pinia";
import { useUiStore } from "@/stores/ui";
import type { Scope } from "@/types/config";

/** Case-insensitive AND match of whitespace-separated terms. */
export function matchesQuery(haystack: string, query: string): boolean {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!terms.length) return true;
  const h = haystack.toLowerCase();
  return terms.every((t) => h.includes(t));
}

/**
 * Filters a list by the global header search + an optional local scope filter.
 * `text` returns the searchable strings for an item.
 */
export function useSearch<T extends { scope?: Scope }>(
  items: Ref<T[]>,
  text: (item: T) => Array<string | null | undefined>,
) {
  const { search } = storeToRefs(useUiStore());
  const scope = ref<Scope | "all">("all");

  const availableScopes = computed(() => {
    const counts = new Map<Scope, number>();
    for (const i of items.value) {
      if (i.scope) counts.set(i.scope, (counts.get(i.scope) ?? 0) + 1);
    }
    return counts;
  });

  const filtered = computed(() =>
    items.value.filter((i) => {
      if (scope.value !== "all" && i.scope !== scope.value) return false;
      return matchesQuery(text(i).filter(Boolean).join(" \n "), search.value);
    }),
  );

  return { query: search, scope, availableScopes, filtered };
}
