<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { Marked } from "marked";
import { escapeHtml, highlight } from "@/lib/highlight";
import { tr } from "@/i18n";

const props = withDefaults(
  defineProps<{ source: string; showFrontmatter?: boolean }>(),
  { showFrontmatter: true },
);

const { t } = useI18n();

function imageAlt(text?: string): string {
  return text ? tr("common.imageAltNamed", { name: text }) : tr("common.imageAlt");
}

// Files on disk are untrusted: raw HTML is escaped (never injected) and only
// http(s)/mailto links are rendered as anchors.
const md = new Marked({
  gfm: true,
  breaks: false,
  renderer: {
    html({ text }) {
      return escapeHtml(text);
    },
    code({ text, lang }) {
      const language = (lang ?? "").split(/\s+/)[0];
      return `<pre><code class="hljs">${highlight(text, language)}</code></pre>`;
    },
    link({ href, title, tokens }) {
      const label = this.parser.parseInline(tokens);
      if (!/^(https?:|mailto:)/i.test(href)) return label;
      const tAttr = title ? ` title="${escapeHtml(title)}"` : "";
      return `<a href="${escapeHtml(href)}"${tAttr} data-external="true">${label}</a>`;
    },
    image({ text }) {
      return escapeHtml(text ? `[${imageAlt(text)}]` : `[${imageAlt()}]`);
    },
  },
});

const parts = computed(() => {
  const src = props.source.replace(/^\uFEFF/, "");
  const m = src.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
  if (!m) return { front: [] as Array<[string, string]>, body: src };
  const front: Array<[string, string]> = [];
  let lastKey: string | null = null;
  for (const line of m[1].split(/\r?\n/)) {
    const kv = line.match(/^([A-Za-z0-9_.-]+):\s*(.*)$/);
    if (kv) {
      lastKey = kv[1];
      front.push([kv[1], kv[2].replace(/^["']|["']$/g, "")]);
    } else if (lastKey && line.trim()) {
      const last = front[front.length - 1];
      last[1] = `${last[1]} ${line.trim()}`.trim();
    }
  }
  return { front, body: src.slice(m[0].length) };
});

const html = computed(() => md.parse(parts.value.body, { async: false }) as string);

async function onClick(e: MouseEvent) {
  const a = (e.target as HTMLElement).closest("a");
  if (!a) return;
  e.preventDefault();
  const href = a.getAttribute("href");
  if (!href || !/^https?:/i.test(href)) return;
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(href);
  } catch {
    /* opener unavailable (browser dev) */
  }
}
</script>

<template>
  <div>
    <div
      v-if="showFrontmatter && parts.front.length"
      class="mb-3 overflow-hidden rounded-md border border-line bg-elevated/50"
    >
      <dl class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-0.5 px-3 py-2 text-xs">
        <template v-for="[k, v] in parts.front" :key="k">
          <dt class="font-mono text-subtle">{{ k }}</dt>
          <dd class="break-words text-fg">{{ v || t("common.emDash") }}</dd>
        </template>
      </dl>
    </div>
    <!-- eslint-disable-next-line vue/no-v-html -->
    <article class="md-body prose prose-sm max-w-none dark:prose-invert" @click="onClick" v-html="html" />
  </div>
</template>
