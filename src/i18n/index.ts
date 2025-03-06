import { createI18n } from 'vue-i18n';
import chs from './chs.json';
import en from './en.json';
import cht from './cht.json';
import jp from './jp.json';

const i18n = createI18n({
  legacy: false,
  locale: 'chs',
  messages: { chs, en, cht, jp },
  globalInjection: true,
  fallbackLocale: 'chs',
});

export default i18n;
