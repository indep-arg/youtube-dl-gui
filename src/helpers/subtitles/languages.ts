import ISO6391 from 'iso-639-1';

export interface SubtitleLanguageOption {
  code: string;
  englishName: string;
  nativeName: string;
}

const codes = ISO6391.getAllCodes();

const expandedCodes: Record<string, SubtitleLanguageOption> = {
  'pt-PT': { code: 'pt-PT', englishName: 'Portuguese (Portugal)', nativeName: 'Português (Portugal)' },
  'pt-BR': { code: 'pt-BR', englishName: 'Portuguese (Brazil)', nativeName: 'Português (Brasil)' },
  'zh-TW': { code: 'zh-TW', englishName: 'Traditional Chinese (Taiwan)', nativeName: '繁體中文（台灣）' },
};

export const languageOptions: SubtitleLanguageOption[] = [...codes, ...Object.keys(expandedCodes)]
  .map((code) => {
    if (expandedCodes[code]) return expandedCodes[code];

    return {
      code,
      englishName: ISO6391.getName(code),
      nativeName: ISO6391.getNativeName(code),
    };
  })
  .sort((a, b) => a.englishName.localeCompare(b.englishName));

export const languageOptionsLookup = new Map(
  languageOptions.map(option => [option.code, option] as const),
);

const displayNamesByLocale = new Map<string, Intl.DisplayNames | null>();

function displayNamesFor(locale: string): Intl.DisplayNames | null {
  if (!displayNamesByLocale.has(locale)) {
    try {
      displayNamesByLocale.set(locale, new Intl.DisplayNames([locale], { type: 'language' }));
    } catch {
      displayNamesByLocale.set(locale, null);
    }
  }
  return displayNamesByLocale.get(locale) ?? null;
}

/** The name of a language in the language of the app, falling back to the English name. */
export function getLocalizedLanguageName(option: SubtitleLanguageOption, locale: string): string {
  if (!locale || locale.toLowerCase().startsWith('en')) {
    return option.englishName;
  }

  try {
    const name = displayNamesFor(locale)?.of(option.code);
    if (name && name.toLowerCase() !== option.code.toLowerCase()) {
      return name.charAt(0).toLocaleUpperCase(locale) + name.slice(1);
    }
  } catch {
    // Not a code Intl knows: use the English name.
  }
  return option.englishName;
}

function normalizeLocale(candidate: string): string {
  const parts = candidate.replace(/_/g, '-').split('-');
  const language = parts[0]?.toLowerCase() ?? '';
  const region = parts[1]?.toUpperCase();

  return region ? `${language}-${region}` : language;
}

export function getSubtitleLanguageLabel(code: string, locale = 'en'): string {
  const trimmed = code.trim();
  if (!trimmed) {
    return code;
  }

  const normalized = trimmed.replace(/_/g, '-');
  const lower = normalized.toLowerCase();
  const isOrig = lower.endsWith('-orig');
  const baseRaw = isOrig ? normalized.slice(0, -5) : normalized;
  const base = normalizeLocale(baseRaw);

  const exact = languageOptionsLookup.get(base)
    || languageOptionsLookup.get(base.toLowerCase());

  const label = exact ? getLocalizedLanguageName(exact, locale) : base.toUpperCase();

  return isOrig ? `${label} (Original audio)` : label;
}

export function getPreferredAutoSubtitleLanguages(codes: string[]): string[] {
  const groups = new Map<string, { plain?: string; orig?: string }>();

  for (const code of codes) {
    const trimmed = code.trim();
    if (!trimmed) continue;

    const normalized = trimmed.replace(/_/g, '-');
    const lower = normalized.toLowerCase();
    const isOrig = lower.endsWith('-orig');
    const base = (isOrig ? normalized.slice(0, -5) : normalized).toLowerCase();
    const existing = groups.get(base) ?? {};

    if (isOrig) existing.orig = lower;
    else existing.plain = lower;

    groups.set(base, existing);
  }

  const hasOrigVariant = [...groups.values()].some(value => value.orig);

  return [...groups.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .filter(([, value]) => !hasOrigVariant || value.orig)
    .map(([, value]) => value.plain ?? value.orig!)
    .filter(Boolean);
}

export function detectBrowserLanguageCodes(): string[] {
  let candidates: readonly string[] = [];

  if (typeof navigator !== 'undefined') {
    candidates = navigator.languages?.length
      ? navigator.languages
      : navigator.language
        ? [navigator.language]
        : [];
  }

  const normalized = new Set<string>();

  for (const candidate of candidates) {
    if (!candidate) continue;

    const locale = normalizeLocale(candidate);

    if (expandedCodes[locale]) {
      normalized.add(locale);
    }

    const isoCode = locale.split('-')[0];
    if (isoCode && ISO6391.validate(isoCode)) {
      normalized.add(isoCode);
    }
  }

  return Array.from(normalized);
}
