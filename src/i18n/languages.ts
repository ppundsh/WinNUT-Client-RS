// Language codes and detection. Kept free of store imports so the store can use these types.

export type Lang = "en" | "zh-TW" | "zh-CN" | "ja";
export type LangSetting = "system" | Lang;

/** Shown in the language picker, each in its own language. */
export const LANGUAGES: { code: Lang; label: string }[] = [
  { code: "zh-TW", label: "繁體中文" },
  { code: "zh-CN", label: "简体中文" },
  { code: "en", label: "English" },
  { code: "ja", label: "日本語" },
];

/** Maps a BCP-47 tag (e.g. from navigator.languages) to a supported language. */
export function matchLang(tag: string | undefined | null): Lang | null {
  if (!tag) return null;
  const t = tag.toLowerCase();
  if (t.startsWith("zh")) {
    // Traditional: Taiwan, Hong Kong, Macau or an explicit Hant script tag.
    if (/-(tw|hk|mo)\b/.test(t) || t.includes("hant")) return "zh-TW";
    return "zh-CN";
  }
  if (t.startsWith("ja")) return "ja";
  if (t.startsWith("en")) return "en";
  return null;
}

export function systemLang(): Lang {
  const tags =
    typeof navigator !== "undefined"
      ? [...(navigator.languages ?? []), navigator.language]
      : [];
  for (const tag of tags) {
    const m = matchLang(tag);
    if (m) return m;
  }
  return "en";
}

export function resolveLang(setting: LangSetting | undefined): Lang {
  if (!setting || setting === "system") return systemLang();
  return LANGUAGES.some((l) => l.code === setting) ? setting : "en";
}
