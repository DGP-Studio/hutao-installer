import { createI18n } from 'vue-i18n';

import chs from './chs.json';
import cht from './cht.json';
import en from './en.json';
import ja from './ja.json';

const getLocale = () => {
  const locale = navigator.language || 'en';
  if (locale.includes('-')) {
    const lang = locale.split('-')[0];
    if (lang.startsWith('zh')) {
      const region = locale.split('-')[1];
      if (region.startsWith('TW') || region.startsWith('HK')) {
        return 'cht';
      }

      return 'chs';
    }
  }

  return locale;
};

const messages: Record<string, Record<string, string>> = { chs, cht, en, ja };
const locale = getLocale();

export const getLang = () => {
  let locale = navigator.language || 'en';
  if (locale.includes('-')) {
    locale = locale.split('-')[0];
  }

  switch (locale) {
    case 'zh':
      return 'zh';
    case 'id':
      return 'id';
    case 'ru':
      return 'ru';
    case 'ja':
      return 'jp';
    default:
      return 'en';
  }
};

export const getLocalizedString = (key: string): string => {
  if (messages[locale] && messages[locale][key]) {
    return messages[locale][key];
  }
  return messages['en'][key] || key; // Fallback to English or return the key itself
};

export const formatLocalizedString = (key: string, ...args: any[]): string => {
  const localizedString = getLocalizedString(key);
  return args.reduce((str, arg, index) => {
    return str.replace(`{${index}}`, arg);
  }, localizedString);
};

const i18n = createI18n({
  legacy: false,
  locale: locale,
  messages: messages,
  globalInjection: true,
  fallbackLocale: 'en',
});

export default i18n;
