import { useCallback } from "react";
import { useUpsStore } from "../store/upsStore";
import en, { type TranslationKey } from "./locales/en";
import zhTW from "./locales/zh-TW";
import zhCN from "./locales/zh-CN";
import ja from "./locales/ja";
import { resolveLang, type Lang } from "./languages";

export type { TranslationKey } from "./locales/en";
export { LANGUAGES, resolveLang, type Lang, type LangSetting } from "./languages";

const DICTS: Record<Lang, Record<TranslationKey, string>> = {
  en,
  "zh-TW": zhTW,
  "zh-CN": zhCN,
  ja,
};

export type TParams = Record<string, string | number | null | undefined>;
export type TFunction = (key: TranslationKey, params?: TParams) => string;

/** Translates `key` for `lang`, replacing `{name}` placeholders. Falls back to English. */
export function translate(lang: Lang, key: TranslationKey, params?: TParams): string {
  const template = DICTS[lang]?.[key] ?? en[key] ?? key;
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (m, name: string) => {
    const v = params[name];
    return v === undefined || v === null ? m : String(v);
  });
}

/** `true` if `key` exists in the dictionaries (used for dynamic keys such as remote commands). */
export function hasKey(key: string): key is TranslationKey {
  return Object.prototype.hasOwnProperty.call(en, key);
}

/** Current UI language, resolved from the persisted setting. */
export function useLang(): Lang {
  const setting = useUpsStore((s) => s.language);
  return resolveLang(setting);
}

/** React hook: returns a translate function bound to the current language. */
export function useT(): TFunction {
  const lang = useLang();
  return useCallback((key: TranslationKey, params?: TParams) => translate(lang, key, params), [lang]);
}

/** Non-hook variant for callbacks outside React render (toasts, notifications). */
export function t(key: TranslationKey, params?: TParams): string {
  return translate(resolveLang(useUpsStore.getState().language), key, params);
}

const NUT_ERROR_CODES = [
  "UNKNOWN-UPS",
  "ACCESS-DENIED",
  "INVALID-USERNAME",
  "INVALID-PASSWORD",
  "USERNAME-REQUIRED",
  "PASSWORD-REQUIRED",
  "DATA-STALE",
  "DRIVER-NOT-CONNECTED",
  "CMD-NOT-SUPPORTED",
] as const;

/** Turns a backend error string into a friendly, localized message. */
export function describeError(tr: TFunction, raw: unknown): string {
  const text = String(raw ?? "");
  for (const code of NUT_ERROR_CODES) {
    if (text.includes(code)) return tr(`err.${code}` as TranslationKey);
  }
  if (/not connected/i.test(text)) return tr("err.notConnected");
  if (/timed? ?out/i.test(text)) return tr("err.timeout");
  return tr("err.generic", { error: text });
}
