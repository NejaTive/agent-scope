import hljs from "highlight.js/lib/core";
import json from "highlight.js/lib/languages/json";
import yaml from "highlight.js/lib/languages/yaml";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import python from "highlight.js/lib/languages/python";
import ini from "highlight.js/lib/languages/ini";
import xml from "highlight.js/lib/languages/xml";
import markdown from "highlight.js/lib/languages/markdown";
import rust from "highlight.js/lib/languages/rust";
import go from "highlight.js/lib/languages/go";
import diff from "highlight.js/lib/languages/diff";

hljs.registerLanguage("json", json);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("python", python);
hljs.registerLanguage("toml", ini);
hljs.registerLanguage("ini", ini);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("markdown", markdown);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("go", go);
hljs.registerLanguage("diff", diff);

const ALIASES: Record<string, string> = {
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  ts: "typescript",
  py: "python",
  yml: "yaml",
  html: "xml",
  md: "markdown",
  jsonc: "json",
  rs: "rust",
};

const MAX_HIGHLIGHT_CHARS = 200_000;

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** Returns HTML-safe highlighted markup (highlight.js escapes its input). */
export function highlight(code: string, lang?: string | null): string {
  if (code.length > MAX_HIGHLIGHT_CHARS) return escapeHtml(code);
  const name = lang ? (ALIASES[lang.toLowerCase()] ?? lang.toLowerCase()) : "";
  try {
    if (name && hljs.getLanguage(name)) {
      return hljs.highlight(code, { language: name, ignoreIllegals: true }).value;
    }
    return escapeHtml(code);
  } catch {
    return escapeHtml(code);
  }
}

export function languageForExt(ext: string): string {
  return ALIASES[ext] ?? ext;
}
