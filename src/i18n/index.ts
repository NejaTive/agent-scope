import { isRef } from "vue";
import { createI18n } from "vue-i18n";
import type { AppLocale } from "@/types/config";
import en from "./locales/en";
import ru from "./locales/ru";

export const LOCALES: AppLocale[] = ["ru", "en"];

export type MessageSchema = typeof ru;

const i18n = createI18n<[MessageSchema], AppLocale>({
  legacy: false,
  locale: "ru",
  fallbackLocale: "ru",
  // `as const` locales have distinct string literal values; structure matches.
  messages: { ru, en: en as unknown as MessageSchema },
});

/** Current UI locale (works whether typings expose a ref or a plain string). */
export function getLocale(): AppLocale {
  const loc = i18n.global.locale as unknown;
  if (isRef(loc)) return loc.value as AppLocale;
  return loc as AppLocale;
}

/** Apply locale to vue-i18n and `<html lang>`. */
export function applyLocale(locale: AppLocale) {
  const loc = i18n.global.locale as unknown;
  if (isRef(loc)) {
    (loc as { value: AppLocale }).value = locale;
  } else {
    (i18n.global as { locale: AppLocale }).locale = locale;
  }
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
  }
}

export function isAppLocale(v: unknown): v is AppLocale {
  return v === "ru" || v === "en";
}

/**
 * Resolve initial locale when prefs have no saved `locale`.
 * `navigator.language` starting with `en` → en; otherwise ru (product default).
 */
export function detectBrowserLocale(): AppLocale {
  try {
    const lang = navigator.language?.toLowerCase() ?? "";
    if (lang.startsWith("en")) return "en";
  } catch {
    /* SSR / non-browser */
  }
  return "ru";
}

/** Translate outside of Vue setup (stores, lib helpers). */
export function tr(key: string, values?: Record<string, unknown>): string {
  return i18n.global.t(key as never, values as never) as string;
}

export default i18n;
