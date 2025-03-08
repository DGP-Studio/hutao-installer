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
}

const i18n = createI18n({
  legacy: false,
  locale: getLocale(),
  messages: { chs, cht, en, ja },
  globalInjection: true,
  fallbackLocale: 'en',
});

export default i18n;
