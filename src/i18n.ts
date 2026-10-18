import { createI18n } from 'vue-i18n';
import en from './locales/en.json';
import { detectBrowserLanguageCodes } from './helpers/subtitles/languages.ts';

export const availableLocales = {
  'en': true,
  'es': true,
  'nl': true,
  'it': true,
  'fr': true,
  'de': true,
  'nb': true,
  'ru': true,
  'tr': true,
  'pt-PT': true,
  'pt-BR': true,
  'zh-TW': true,
  'ko': true,
} as const;

type MessageSchema = typeof en;
type Locale = keyof typeof availableLocales;

// English is always there (it is also the fallback); the other languages are
// loaded when they are used, so the app does not carry all of them at start.
const loaders = import.meta.glob<{ default: MessageSchema }>(['./locales/*.json', '!./locales/en.json']);

const localeAliases: Record<string, Locale> = {
  'pt': 'pt-PT',
  'pt-PT': 'pt-PT',
  'pt-BR': 'pt-BR',
  'zh': 'zh-TW',
  'zh-Hant': 'zh-TW',
  'zh-TW': 'zh-TW',
  'no': 'nb',
  'nb-NO': 'nb',
};

export function getDefaultLocale(): Locale {
  for (const code of detectBrowserLanguageCodes()) {
    if (code in availableLocales) {
      return code as Locale;
    }

    if (code in localeAliases) {
      return localeAliases[code];
    }

    const baseCode = code.split('-')[0];

    if (baseCode in availableLocales) {
      return baseCode as Locale;
    }

    if (baseCode in localeAliases) {
      return localeAliases[baseCode];
    }
  }

  return 'en';
}

export const i18n = createI18n<[MessageSchema], Locale, false>({
  locale: 'en',
  legacy: false,
  globalInjection: false,
  fallbackLocale: 'en',
  messages: { en } as Record<Locale, MessageSchema>,
});

/** The locale for a language setting, which can also be "system". */
export function resolveLocale(language: string): Locale {
  return language === 'system' || !(language in availableLocales)
    ? getDefaultLocale()
    : language as Locale;
}

/** Loads the messages of a language if needed, then switches to it. */
export async function setLocale(locale: Locale): Promise<void> {
  const loaded = i18n.global.availableLocales.includes(locale);
  const loader = loaders[`./locales/${locale}.json`];
  if (!loaded && loader) {
    try {
      const messages = await loader();
      i18n.global.setLocaleMessage(locale, messages.default);
    } catch (e) {
      console.error(`Unable to load the ${locale} translation:`, e);
      return;
    }
  }
  i18n.global.locale.value = locale;
  document.documentElement.setAttribute('lang', locale);
}
