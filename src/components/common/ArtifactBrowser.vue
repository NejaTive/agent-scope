<script setup lang="ts">
import { computed, ref, toRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { storeToRefs } from "pinia";
import { useSearch } from "@/composables/useSearch";
import { useUiStore } from "@/stores/ui";
import {
  KIND_LABEL,
  KIND_PLURAL,
  PRODUCT_LABEL,
  PRODUCT_ORDER,
  SCOPE_LABEL,
  SCOPE_ORDER,
  SCOPE_PRECEDENCE,
  STATUS_LABEL,
  STATUS_TONE,
  resolveProduct,
} from "@/lib/labels";
import { relativeTime, shortenPath } from "@/lib/format";
import { isProductTracked } from "@/lib/tooling";
import type { BrowserItem } from "@/lib/browser";
import type { ArtifactKind, Product, Scope } from "@/types/config";
import { usePreferencesStore } from "@/stores/preferences";
import ArtifactDetail from "./ArtifactDetail.vue";
import EmptyState from "./EmptyState.vue";
import LoadingBlock from "./LoadingBlock.vue";
import ProductBadge from "./ProductBadge.vue";
import ScopeBadge from "./ScopeBadge.vue";
import StatusDot from "./StatusDot.vue";
import Badge from "./Badge.vue";

const props = withDefaults(
  defineProps<{
    items: BrowserItem[];
    loading?: boolean;
    emptyTitle?: string;
    emptyDescription?: string;
    /** Show per-kind filter chips (for mixed-kind inventories). */
    kindFilter?: boolean;
    /** Show the kind badge on each row. */
    showKind?: boolean;
    /** Force product badge even when only one product is present. */
    showProduct?: boolean;
    collapsedContent?: boolean;
    contentLabel?: string;
    listWidth?: number;
  }>(),
  {
    loading: false,
    emptyTitle: undefined,
    emptyDescription: undefined,
    kindFilter: false,
    showKind: false,
    showProduct: false,
    collapsedContent: false,
    contentLabel: undefined,
    listWidth: 340,
  },
);

const { t, locale } = useI18n();
const resolvedEmptyTitle = computed(() => props.emptyTitle ?? t("empty.defaultTitle"));
const resolvedEmptyDescription = computed(
  () => props.emptyDescription ?? t("browser.emptyDescription"),
);
const resolvedContentLabel = computed(() => props.contentLabel ?? t("artifact.contentLabel"));

const ui = useUiStore();
const prefs = usePreferencesStore();
const { focusArtifactId } = storeToRefs(ui);

const product = ref<Product | "all">("all");
const kind = ref<string>("all");

const trackedItems = computed(() =>
  props.items.filter((i) => isProductTracked(prefs.prefs, resolveProduct(i.product))),
);

const productCounts = computed(() => {
  const m = new Map<Product, number>();
  for (const i of trackedItems.value) {
    const p = resolveProduct(i.product);
    m.set(p, (m.get(p) ?? 0) + 1);
  }
  return m;
});

const showProductFilter = computed(() => productCounts.value.size > 1);
const showProductBadge = computed(() => props.showProduct || showProductFilter.value);

const productScoped = computed(() =>
  product.value === "all"
    ? trackedItems.value
    : trackedItems.value.filter((i) => resolveProduct(i.product) === product.value),
);

const kindLabel = (k: string) => KIND_PLURAL[k as ArtifactKind] ?? k;

const kindCounts = computed(() => {
  const m = new Map<string, number>();
  for (const i of productScoped.value) if (i.kind) m.set(String(i.kind), (m.get(String(i.kind)) ?? 0) + 1);
  return m;
});

const kindOptions = computed(() =>
  [...kindCounts.value.entries()]
    .map(([k, n]) => ({ kind: k, count: n, label: kindLabel(k) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
);

const kindScoped = computed(() =>
  kind.value === "all"
    ? productScoped.value
    : productScoped.value.filter((i) => String(i.kind) === kind.value),
);

const { scope, availableScopes, filtered } = useSearch(kindScoped, (i) => [
  i.name,
  i.subtitle,
  i.path,
  i.projectPath,
  i.searchText,
  PRODUCT_LABEL[resolveProduct(i.product)],
]);

const sorted = computed(() =>
  [...filtered.value].sort(
    (a, b) =>
      (SCOPE_PRECEDENCE[b.scope] ?? 0) - (SCOPE_PRECEDENCE[a.scope] ?? 0) ||
      a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
  ),
);

const productChips = computed(() =>
  PRODUCT_ORDER.filter((p) => productCounts.value.has(p)).map((p) => ({
    product: p,
    count: productCounts.value.get(p) ?? 0,
  })),
);

const scopeOptions = computed(() =>
  SCOPE_ORDER.filter((s) => availableScopes.value.has(s)).map((s) => ({
    scope: s,
    count: availableScopes.value.get(s) ?? 0,
    label: SCOPE_LABEL[s],
  })),
);

const showKindSelect = computed(() => props.kindFilter && kindOptions.value.length > 1);
const showScopeSelect = computed(() => scopeOptions.value.length > 1);

const filtersActive = computed(
  () => product.value !== "all" || kind.value !== "all" || scope.value !== "all",
);

function resetFilters() {
  product.value = "all";
  kind.value = "all";
  scope.value = "all";
}

// Drop stale kind/scope when the available options shrink.
watch(kindOptions, (opts) => {
  if (kind.value !== "all" && !opts.some((o) => o.kind === kind.value)) kind.value = "all";
});
watch(scopeOptions, (opts) => {
  if (scope.value !== "all" && !opts.some((o) => o.scope === scope.value)) scope.value = "all";
});

const selectedId = ref<string | null>(null);
const selected = computed(() => sorted.value.find((i) => i.id === selectedId.value) ?? null);

// Keep a valid selection as filters / data change.
watch(
  sorted,
  (list) => {
    if (!list.length) selectedId.value = null;
    else if (!list.some((i) => i.id === selectedId.value)) selectedId.value = list[0].id;
  },
  { immediate: true },
);

// Honour "open this artifact" requests coming from global search.
watch(
  [focusArtifactId, toRef(props, "items")],
  () => {
    const id = focusArtifactId.value;
    if (!id) return;
    const hit = props.items.find((i) => i.id === id || i.artifactId === id);
    if (hit) {
      product.value = "all";
      kind.value = "all";
      scope.value = "all";
      selectedId.value = hit.id;
      focusArtifactId.value = null;
    }
  },
  { immediate: true },
);

</script>

<template>
  <div class="flex h-full min-h-0">
    <section
      class="flex min-h-0 shrink-0 flex-col border-r border-line"
      :style="{ width: `${listWidth}px` }"
    >
      <slot name="header" />

      <div class="shrink-0 space-y-1.5 border-b border-line px-2 py-1.5">
        <div v-if="showProductFilter" class="flex flex-wrap gap-0.5">
          <button
            type="button"
            class="chip"
            :class="product === 'all' && 'chip-active'"
            @click="product = 'all'"
          >
            {{ t("common.all") }}
          </button>
          <button
            v-for="c in productChips"
            :key="c.product"
            type="button"
            class="chip"
            :class="product === c.product && 'chip-active'"
            @click="product = c.product"
          >
            {{ PRODUCT_LABEL[c.product] }}
          </button>
        </div>
        <div class="flex flex-wrap items-center gap-1.5">
          <select
            v-if="showKindSelect"
            class="input max-w-[9.5rem] shrink-0"
            :value="kind"
            @change="kind = ($event.target as HTMLSelectElement).value"
          >
            <option value="all">{{ t("browser.allKinds") }}</option>
            <option v-for="o in kindOptions" :key="o.kind" :value="o.kind">
              {{ o.label }}
            </option>
          </select>
          <select
            v-if="showScopeSelect"
            class="input max-w-[9.5rem] shrink-0"
            :value="scope"
            @change="scope = ($event.target as HTMLSelectElement).value as Scope | 'all'"
          >
            <option value="all">{{ t("browser.allScopes") }}</option>
            <option v-for="o in scopeOptions" :key="o.scope" :value="o.scope">
              {{ o.label }}
            </option>
          </select>
          <button
            v-if="filtersActive"
            type="button"
            class="btn-ghost !h-7 !px-1.5 text-2xs"
            :title="t('browser.resetFilters')"
            @click="resetFilters"
          >
            {{ t("common.reset") }}
          </button>
          <span class="ml-auto pr-1 font-mono text-2xs text-subtle">{{ sorted.length }}</span>
        </div>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto">
        <LoadingBlock v-if="loading && !items.length" :lines="6" />
        <EmptyState
          v-else-if="!sorted.length"
          compact
          :title="items.length ? t('browser.noMatches') : resolvedEmptyTitle"
          :description="items.length ? t('browser.noMatchesHint') : resolvedEmptyDescription"
        />
        <ul v-else class="py-1" role="listbox">
          <li v-for="item in sorted" :key="item.id" role="option" :aria-selected="item.id === selectedId">
            <button
              type="button"
              class="flex w-full flex-col gap-0.5 border-l-2 px-3 py-1.5 text-left transition-colors"
              :class="
                item.id === selectedId
                  ? 'border-accent bg-elevated'
                  : 'border-transparent hover:bg-elevated/50'
              "
              @click="selectedId = item.id"
            >
              <span class="flex items-center gap-1.5">
                <StatusDot
                  v-if="item.status && item.status !== 'present'"
                  :tone="STATUS_TONE[item.status] === 'neutral' ? 'neutral' : STATUS_TONE[item.status]"
                  :title="STATUS_LABEL[item.status]"
                />
                <span class="min-w-0 flex-1 truncate text-[13px] font-medium">{{ item.name }}</span>
                <Badge v-if="showKind && item.kind">{{ KIND_LABEL[item.kind as ArtifactKind] ?? item.kind }}</Badge>
                <ProductBadge v-if="showProductBadge" :product="item.product" />
                <ScopeBadge :scope="item.scope" />
              </span>
              <span v-if="item.subtitle" class="line-clamp-1 text-xs text-muted">{{ item.subtitle }}</span>
              <span class="flex items-center gap-2 text-2xs text-subtle">
                <span class="min-w-0 flex-1 truncate font-mono" :title="item.path">{{ shortenPath(item.path) }}</span>
                <span class="shrink-0">{{ relativeTime(item.mtime) }}</span>
              </span>
            </button>
          </li>
        </ul>
      </div>
    </section>

    <section class="min-w-0 flex-1">
      <ArtifactDetail
        v-if="selected"
        :key="selected.id"
        :item="selected"
        :collapsed-content="collapsedContent"
        :content-label="resolvedContentLabel"
      >
        <template #extra>
          <slot name="detail" :item="selected" />
        </template>
      </ArtifactDetail>
      <EmptyState
        v-else
        :title="t('browser.selectItem')"
        :description="t('browser.selectItemHint')"
        icon="eye"
      />
    </section>
  </div>
</template>
