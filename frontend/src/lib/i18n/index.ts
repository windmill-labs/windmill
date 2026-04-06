export const SUPPORTED_LOCALES = ['en', 'fr', 'de', 'es', 'pt', 'ja', 'zh', 'ko'] as const
export type Locale = (typeof SUPPORTED_LOCALES)[number]
export const DEFAULT_LOCALE: Locale = 'en'

export const LOCALE_NAMES: Record<Locale, string> = {
	en: 'English',
	fr: 'Français',
	de: 'Deutsch',
	es: 'Español',
	pt: 'Português',
	ja: '日本語',
	zh: '中文',
	ko: '한국어'
}
