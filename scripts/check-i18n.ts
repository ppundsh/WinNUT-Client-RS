// Verifies the translation files: same keys as English (also enforced by tsc), no empty
// strings, and identical {placeholders} in every language.
// Run: bun run scripts/check-i18n.ts
import en from "../src/i18n/locales/en";
import zhTW from "../src/i18n/locales/zh-TW";
import zhCN from "../src/i18n/locales/zh-CN";
import ja from "../src/i18n/locales/ja";
import { matchLang } from "../src/i18n/languages";

const locales: Record<string, Record<string, string>> = { "zh-TW": zhTW, "zh-CN": zhCN, ja };
const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(",");

let problems = 0;
const fail = (msg: string) => {
  problems++;
  console.error("  ✗ " + msg);
};

for (const [code, dict] of Object.entries(locales)) {
  for (const key of Object.keys(en) as (keyof typeof en)[]) {
    const value = dict[key];
    if (value === undefined) fail(`${code}: missing "${key}"`);
    else if (value.trim() === "") fail(`${code}: empty "${key}"`);
    else if (placeholders(value) !== placeholders(en[key]))
      fail(`${code}: "${key}" placeholders {${placeholders(value)}} ≠ English {${placeholders(en[key])}}`);
  }
  for (const key of Object.keys(dict)) {
    if (!(key in en)) fail(`${code}: unknown key "${key}"`);
  }
}

const cases: [string, string | null][] = [
  ["zh-TW", "zh-TW"], ["zh-Hant-HK", "zh-TW"], ["zh-HK", "zh-TW"], ["zh-MO", "zh-TW"],
  ["zh-CN", "zh-CN"], ["zh", "zh-CN"], ["zh-Hans-SG", "zh-CN"],
  ["ja-JP", "ja"], ["en-US", "en"], ["fr-FR", null],
];
for (const [tag, want] of cases) {
  if (matchLang(tag) !== want) fail(`matchLang(${tag}) = ${matchLang(tag)}, expected ${want}`);
}

const total = Object.keys(en).length;
if (problems > 0) {
  console.error(`i18n check failed: ${problems} problem(s).`);
  process.exit(1);
}
console.log(`i18n check passed: ${total} keys × ${Object.keys(locales).length + 1} languages.`);
